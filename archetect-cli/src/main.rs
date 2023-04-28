use camino::Utf8PathBuf;
use std::error::Error;
use std::fs;
use std::ops::Deref;
use std::str::FromStr;

use clap::ArgMatches;
use linked_hash_map::LinkedHashMap;
use log::{error, info, warn};
use rhai::plugin::RhaiResult;
use rhai::{Dynamic, EvalAltResult, Map};

use archetect_core::config::{AnswerConfig, AnswerInfo, Catalog, CatalogEntry, CATALOG_FILE_NAME};
use archetect_core::input::select_from_catalog;
use archetect_core::source::Source;
use archetect_core::v2::archetype::archetype_context::ArchetypeContext;
use archetect_core::v2::runtime::context::RuntimeContext;
use archetect_core::Archetect;
use archetect_core::{self, ArchetectError};

pub mod answers;
mod cli;
pub mod vendor;

fn main() {
    let matches = cli::command().get_matches();

    cli::configure(&matches);

    match execute_2(matches) {
        Ok(()) => (),
        Err(error) => {
            error!("{}", error);
            std::process::exit(-1);
        }
    }
}

fn execute_2(matches: ArgMatches) -> Result<(), ArchetectError> {
    let mut answers = Map::new();

    if let Some(answer_files) = matches.get_many::<String>("answer-file") {
        for answer_file in answer_files {
            let results = answers::read_answers(answer_file)?;
            answers.extend(results);
        }
    }

    if let Some(answer_matches) = matches.get_many::<String>("answer") {
        let engine = rhai::Engine::new();
        for answer_match in answer_matches {
            let (identifier, value) = archetect_core::config::answers::parse_answer_pair(answer_match).unwrap();
            let result: Result<Dynamic, Box<EvalAltResult>> = engine.eval(&value);
            match result {
                Ok(value) => {
                    answers.insert(identifier.into(), value);
                }
                Err(err) => match err.deref() {
                    EvalAltResult::ErrorVariableNotFound(_, _) => {
                        let result: Result<Dynamic, Box<EvalAltResult>> = engine.eval(format!("\"{}\"", &value).as_str());
                        match result {
                            Ok(value) => {
                                answers.insert(identifier.into(), value);
                            }
                            Err(err) => {
                                return Err(err.into());
                            }
                        }
                    }
                    _ => return Err(err.into()),
                },
            }
        }

    }

    match matches.subcommand() {
        None => {}
        Some(("completions", args)) => cli::completions(args)?,
        Some(("render", args)) => render(args, answers)?,
        _ => {}
    }

    Ok(())
}

pub fn render(matches: &ArgMatches, answers: Map) -> Result<(), ArchetectError> {
    let source = matches.get_one::<String>("source").unwrap();
    let source = archetect_core::v2::source::Source::detect(&Archetect::build()?, source, None)?;
    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

    let mut archetype = archetect_core::v2::archetype::archetype::Archetype::new(&source)?;
    let mut runtime_context = RuntimeContext::default();
    runtime_context.set_local(matches.get_flag("local"));
    runtime_context.set_headless(matches.get_flag("headless"));
    runtime_context.set_offline(matches.get_flag("offline"));
    if let Some(switches) = matches.get_many::<String>("switches") {
        for switch in switches {
            runtime_context.enable_switch(switch);
        }
    }

    archetype.render_with_destination(destination, runtime_context, answers)?;
    Ok(())
}
