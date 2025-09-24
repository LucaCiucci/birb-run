use std::{any::Any, borrow::Cow, path::{Path, PathBuf}};

use crate::task::{AbstractTaskfileSource, AbstractTaskfileSourceExt, Taskfile, TaskfileLoader, TaskfileLoadError};


pub const YAML_DATA_EXTENSIONS: &[&str] = &["yml", "yaml", "json"];

/// Yaml file frontend
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YamlTaskfileLoader;

impl TaskfileLoader for YamlTaskfileLoader {
    fn find_taskfile(
        &self,
        path: &Path,
    ) -> Option<Box<dyn AbstractTaskfileSource>> {

        let search_paths = std::iter::once(path.to_path_buf()) // TODO use Cow
            .chain(
                YAML_DATA_EXTENSIONS
                    .iter()
                    // don't waste time trying to append filenames if this was not a directory
                    .take(if path.is_dir() { YAML_DATA_EXTENSIONS.len() } else { 0 })
                    .map(|ext| path.join(format!("tasks.{ext}")))
            );

        for path in search_paths {
            if let Some(source) = maybe_yaml_source_from_file(&path) {
                return Some(Box::new(source));
            }
        }

        None
    }

    fn load_taskfile(
        &self,
        source: Box<dyn AbstractTaskfileSource>,
    ) -> Result<Taskfile, TaskfileLoadError> {
        let source: &YamlTaskfileSource = source.downcast_load()?;
        Ok(Taskfile::from_yaml_file(&source.0)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YamlTaskfileSource(PathBuf);

impl AbstractTaskfileSource for YamlTaskfileSource {
    fn path<'s>(&'s self) -> Cow<'s, Path> {
        Cow::Borrowed(&self.0)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn maybe_yaml_source_from_file(path: &Path) -> Option<YamlTaskfileSource> {
    if !path.is_file() {
        return None;
    }

    // check extension
    if !YAML_DATA_EXTENSIONS.contains(&path.extension().and_then(|e| e.to_str())?) {
        return None;
    }

    Some(YamlTaskfileSource(path.to_path_buf().canonicalize().ok()?))
}
