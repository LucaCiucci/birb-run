mod instantiation;

pub use instantiation::{ArgumentsCheckError, InstantiationError};

mod from_yaml;
mod loader;
mod invocation;
mod params;
mod task_ref;
mod task;
mod taskfile;
mod workspace;

pub use loader::*;
pub use invocation::*;
pub use params::*;
use serde::Serialize;
pub use task_ref::*;
pub use task::*;
pub use taskfile::*;
pub use workspace::*;

#[derive(Serialize)]
pub struct BirbRenderContext<Args, Env> {
    pub args: Args,
    pub env: Env,
}