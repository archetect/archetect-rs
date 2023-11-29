mod terminal_io_driver;
mod text_prompt_handler;
mod int_prompt_handler;
mod bool_prompt_handler;
mod list_prompt_handler;
mod select_prompt_handler;
mod multiselect_prompt_handler;

use inquire::ui::{Color, RenderConfig, Styled};
pub use terminal_io_driver::TerminalIoDriver;

pub(crate) fn get_render_config() -> RenderConfig {
    RenderConfig::default_colored()
        .with_canceled_prompt_indicator(Styled::new("<none>").with_fg(Color::DarkGrey))
}