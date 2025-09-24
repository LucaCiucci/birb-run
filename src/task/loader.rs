use std::{any::Any, borrow::Cow, fmt::Debug, path::Path};

use crate::task::{Taskfile, YamlLoadError};

pub mod yaml;
pub mod yaml_executable;

pub trait TaskfileLoader: Debug {
    fn find_taskfile(
        &self,
        path: &Path,
    ) -> Option<Box<dyn AbstractTaskfileSource>>;

    fn load_taskfile(
        &self,
        source: Box<dyn AbstractTaskfileSource>,
    ) -> Result<Taskfile, TaskfileLoadError>;
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum TaskfileLoadError {
    #[error("Invalid source type")]
    InvalidType,
    #[error("Failed to load taskfile from YAML: {0}")]
    YamlLoader(#[from] YamlLoadError),
    #[error("Failed to load taskfile: {0}")]
    Other(#[from] anyhow::Error),
}

// TODO rename TaskfileSource and replace the old one
pub trait AbstractTaskfileSource: Debug + Send + Sync + 'static {
    fn path<'s>(&'s self) -> Cow<'s, Path>;
    fn as_any(&self) -> &dyn Any;
}

pub trait AbstractTaskfileSourceExt: AbstractTaskfileSource {
    /// Downcasts the source to the specified type, returning an error if the type does not match.
    fn downcast_load<T: 'static>(&self) -> Result<&T, TaskfileLoadError> {
        self.as_any().downcast_ref::<T>().ok_or(TaskfileLoadError::InvalidType)
    }
}

impl<T: AbstractTaskfileSource + ?Sized> AbstractTaskfileSourceExt for T {}
