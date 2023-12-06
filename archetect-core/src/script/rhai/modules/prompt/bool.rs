use rhai::{Dynamic, EvalAltResult, Map, NativeCallContext};

use archetect_api::{BoolPromptInfo, CommandRequest, CommandResponse, PromptInfo};

use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::runtime::context::RuntimeContext;
use crate::script::rhai::modules::prompt::{cast_setting, extract_prompt_info};

// TODO: Better help messages
pub fn prompt<'a, K: AsRef<str> + Clone>(
    call: &NativeCallContext,
    message: &str,
    runtime_context: &RuntimeContext,
    settings: &Map,
    key: Option<K>,
    answer: Option<&Dynamic>,
) -> Result<Option<bool>, Box<EvalAltResult>> {
    let default = cast_setting("defaults_with", settings, message, key.clone())
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    let mut prompt_info = BoolPromptInfo::new(message, key)
        .with_default(default)
        ;

    extract_prompt_info(&mut prompt_info, settings)
        .map_err(|error| ArchetypeScriptErrorWrapper(call, error))?;

    if runtime_context.headless() {
        if let Some(default) = prompt_info.default() {
            return Ok(Some(default));
        } else if prompt_info.optional() {
            return Ok(None)
        }
        let error = ArchetypeScriptError::headless_no_answer(&prompt_info);
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if let Some(answer) = answer {
        return match get_boolean(answer.to_string().as_str()) {
            Ok(value) => Ok(value.into()),
            Err(_) => {
                let error = ArchetypeScriptError::answer_validation_error(
                    answer.to_string(),
                    &prompt_info,
                    "must resemble a boolean",
                );
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            }
        };
    }

    runtime_context.request(CommandRequest::PromptForBool(prompt_info.clone()));

    match runtime_context.response() {
        CommandResponse::Boolean(answer) => {
            return Ok(Some(answer));
        }
        CommandResponse::None => {
            if !prompt_info.optional() {
                let error = ArchetypeScriptError::answer_not_optional(&prompt_info);
                return Err(ArchetypeScriptErrorWrapper(call, error).into());
            } else {
                return Ok(None);
            }
        }
        CommandResponse::Error(error) => {
            return Err(ArchetypeScriptErrorWrapper(call, ArchetypeScriptError::PromptError(error)).into());
        }
        response => {
            let error = ArchetypeScriptError::unexpected_prompt_response(&prompt_info, "a Boolean", response);
            return Err(ArchetypeScriptErrorWrapper(call, error).into());
        }
    }
}

fn get_boolean<V: AsRef<str>>(value: V) -> Result<bool, ()> {
    match value.as_ref().to_lowercase().as_str() {
        "y" | "yes" | "t" | "true" => Ok(true),
        "n" | "no" | "f" | "false" => Ok(false),
        _ => Err(()),
    }
}
