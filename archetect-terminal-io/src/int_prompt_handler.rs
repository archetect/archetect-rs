use archetect_api::{ClientIoHandle, ClientMessage, IntPromptInfo, PromptInfo, PromptInfoLengthRestrictions};
use archetect_inquire::{InquireError, Text};
use archetect_inquire::validator::Validation;
use archetect_validations::validate_int_size;

use crate::get_render_config;

pub fn handle_prompt_int<CIO: ClientIoHandle>(prompt_info: IntPromptInfo, client_handle: &CIO) {
    let mut prompt = Text::new(prompt_info.message()).with_render_config(get_render_config());
    let default = prompt_info.default().map(|v| v.to_string());
    prompt.default = default;
    prompt.placeholder = prompt_info.placeholder().map(|v| v.to_string());
    if prompt_info.help().is_some() {
        prompt.help_message = prompt_info.help().map(|v| v.to_string());
    }
    let prompt_info = prompt_info.clone();
    let validator = move |input: &str| match validate(prompt_info.min(), prompt_info.max(), input) {
        Ok(_) => Ok(Validation::Valid),
        Err(message) => Ok(Validation::Invalid(message.into())),
    };
    prompt = prompt.with_validator(validator);
    match prompt.prompt_skippable() {
        Ok(answer) => {
            if let Some(answer) = answer {
                client_handle.send(ClientMessage::Integer(answer.parse::<i64>().expect("Pre-validated")));
            } else {
                client_handle.send(ClientMessage::None);
            }
        }
        Err(error) => match error {
            InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                client_handle.send(ClientMessage::Abort);
            }
            _ => {
                client_handle.send(ClientMessage::Error(error.to_string()));
            }
        },
    }
}

fn validate(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    match input.parse::<i64>() {
        Ok(value) => validate_int_size(min, max, value),
        Err(_) => Err(format!("{} is not an 'int'", input)),
    }
}
