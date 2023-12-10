use std::collections::HashSet;

use camino::Utf8PathBuf;
use clap::ArgMatches;
use rhai::{Dynamic, Map};

use archetect_api::{CommandRequest, IoDriver};
use archetect_core::{self};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::{ArchetectError, ArchetypeError};
use archetect_core::runtime::context::RuntimeContext;
use archetect_core::system::{RootedSystemLayout, SystemLayout};
use archetect_terminal_io::TerminalIoDriver;
use ArchetypeError::ScriptAbortError;

use crate::answers::parse_answer_pair;

mod answers;
mod cli;
mod configuration;
pub mod vendor;
mod subcommands;

fn main() {
    let matches = cli::command().get_matches();
    cli::configure(&matches);

    let driver = TerminalIoDriver::default();
    let layout = RootedSystemLayout::dot_home().unwrap();

    match execute(matches, driver.clone(), layout) {
        Ok(()) => (),
        Err(error) => {
            match error {
                // Handled when the Rhai script ends by the IO Driver
                ArchetectError::ArchetypeError(ScriptAbortError) => {}
                _ => {
                    driver.send(CommandRequest::LogError(format!("{}", error)));
                }
            }

            std::process::exit(-1);
        }
    }
}

fn execute<D: IoDriver, L: SystemLayout>(matches: ArgMatches, driver: D, layout: L) -> Result<(), ArchetectError> {
    let configuration = configuration::load_user_config(&layout, &matches)
        .map_err(|err| ArchetectError::GeneralError(err.to_string()))?;

    let mut answers = Map::new();
    // Load answers from merged configuration
    for (identifier, value) in configuration.answers() {
        answers.insert(identifier.clone(), value.clone());
    }

    // Load answers from answer files
    if let Some(answer_files) = matches.get_many::<String>("answer-file") {
        for answer_file in answer_files {
            let results = answers::read_answers(answer_file)?;
            answers.extend(results);
        }
    }

    // Load answers from individual answer arguments
    if let Some(answer_matches) = matches.get_many::<String>("answer") {
        for answer_match in answer_matches {
            let (identifier, value) = parse_answer_pair(answer_match).unwrap();
            if let Ok(value) = value.parse::<i64>() {
                answers.insert(identifier.into(), value.into());
            } else if let Ok(value) = value.parse::<bool>() {
                answers.insert(identifier.into(), value.into());
            } else {
                answers.insert(identifier.into(), value.into());
            }
        }
    }

    let runtime_context = RuntimeContext::builder()
        .with_configuration(configuration)
        .with_driver(driver)
        .with_layout(layout)
        .build()?;


    match matches.subcommand() {
        None => {
            default(&matches, runtime_context, answers)?;
        }
        Some(("completions", args)) => cli::completions(args)?,
        Some(("render", args)) => render(args, runtime_context, answers).map(|_| ())?,
        Some(("catalog", args)) => catalog(args, runtime_context, answers).map(|_| ())?,
        Some(("config", args)) => config(args, runtime_context.configuration()).map(|_| ())?,
        Some(("cache", args)) => subcommands::handle_cache_subcommand(args, &runtime_context)?,
        _ => {}
    }

    Ok(())
}

fn config(matches: &ArgMatches, configuration: &Configuration) -> Result<(), ArchetectError> {
    match matches.subcommand() {
        Some(("merged", _args)) => {
            println!("{}", configuration.to_yaml());
        }
        Some(("defaults", _args)) => {
            println!("{}", Configuration::default().to_yaml());
        }
        None => {}
        _ => {}
    }

    Ok(())
}

fn default(
    matches: &ArgMatches,
    runtime_context: RuntimeContext,
    answers: Map,
) -> Result<(), ArchetectError> {
    let catalog = runtime_context.configuration().catalog();
    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());
    let render_context = RenderContext::new(destination, answers).with_switches(get_switches(matches, runtime_context.configuration()));
    catalog.render(runtime_context, render_context)?;
    Ok(())
}

fn catalog(
    matches: &ArgMatches,
    runtime_context: RuntimeContext,
    answers: Map,
) -> Result<(), ArchetectError> {
    let source = matches.get_one::<String>("source").unwrap();
    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

    let catalog = runtime_context.new_catalog(source, false)?;
    catalog.check_requirements(&runtime_context)?;
    let render_context = RenderContext::new(destination, answers).with_switches(get_switches(matches, runtime_context.configuration()));
    catalog.render(runtime_context, render_context)?;
    Ok(())
}

pub fn render(
    matches: &ArgMatches,
    runtime_context: RuntimeContext,
    answers: Map,
) -> Result<Dynamic, ArchetectError> {
    let source = matches.get_one::<String>("source").unwrap();
    let archetype = runtime_context.new_archetype(source, false)?;

    let destination = Utf8PathBuf::from(matches.get_one::<String>("destination").unwrap());

    archetype.check_requirements(&runtime_context)?;
    let render_context = RenderContext::new(destination, answers).with_switches(get_switches(matches, runtime_context.configuration()));
    Ok(archetype.render(runtime_context, render_context)?)
}

fn get_switches(matches: &ArgMatches, configuration: &Configuration) -> HashSet<String> {
    let mut switches = HashSet::new();
    for switch in configuration.switches() {
        switches.insert(switch.to_string());
    }
    if let Some(answer_switches) = matches.get_many::<String>("switches") {
        for switch in answer_switches {
            switches.insert(switch.to_string());
        }
    }
    switches
}
