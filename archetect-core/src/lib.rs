#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate pest_derive;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate serde_derive;

pub use crate::archetype::ArchetypeError;
pub use crate::core::Archetect;
pub use crate::errors::{ArchetectError, RenderError};

mod core;
mod errors;

pub mod archetype;
pub mod config;
pub mod requirements;
pub mod rules;
pub mod system;
mod utils;
pub mod v2;
pub mod vendor;
