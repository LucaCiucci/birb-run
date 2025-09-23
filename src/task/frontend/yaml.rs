use std::{any::Any, borrow::Cow, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

use crate::task::{AbstractTaskfileSource, AbstractTaskfileSourceExt, Taskfile, TaskfileFrontend, TaskfileLoadError, TaskfileSource};


pub const YAML_DATA_EXTENSIONS: &[&str] = &["yml", "yaml", "json"];

/// Yaml file frontend
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YamlFrontend;

impl TaskfileFrontend for YamlFrontend {
    fn find_taskfile_in_dir(
        &self,
        path: &Path,
    ) -> Option<Box<dyn AbstractTaskfileSource>> {

        let search_paths = std::iter::once(path.to_path_buf()) // TODO use Cow
            .chain(
                YAML_DATA_EXTENSIONS
                    .iter()
                    // don't waste time trying to append filenames if this was not a directory
                    .take(if path.is_dir() { YAML_DATA_EXTENSIONS.len() } else { 0 })
                    .map(|ext| path.join(format!("task.{ext}")))
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

        todo!()
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

    // check stem
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !(stem == "task" || stem == "tasks") {
        return None;
    }

    Some(YamlTaskfileSource(path.to_path_buf().canonicalize().ok()?))
}

fn maybe_source_from_file(path: &Path, check_stem2: bool) -> Option<TaskfileSource> {
    if !path.is_file() {
        return None;
    }

    if path.extension().map(|ext| YAML_DATA_EXTENSIONS.contains(&ext.to_str().unwrap_or(""))).unwrap_or(false) {
        // TODO error handling with WorkspaceLoadError::Canonicalize
        // TODO maybe canonicalize is not actually needed here, it will be done in workspace.rs anyway
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if check_stem2 && !(stem == "task" || stem == "tasks") {
            return None;
        }
        return Some(TaskfileSource::YamlFile(path.to_path_buf().canonicalize().ok()?));
    }

    // check if it is an executable file in the form <something>.yaml.<ext>
    if let Some(stem) = path.file_stem() {
        let Some(stem) = stem.to_str() else {
            // TODO error handling
            panic!("Failed to convert file stem to str: {:?}", stem);
        };

        let is_correct_ext = 'e: {
            for ext in YAML_DATA_EXTENSIONS {
                // check if it ends with .yaml.<ext> or .yml.<ext> or .json.<ext>
                if !(stem.ends_with(ext) && stem.strip_suffix(ext).map(|s| s.ends_with('.')).unwrap_or(false)){
                    continue;
                }

                let stem2 = stem.strip_suffix(&format!(".{ext}")).unwrap_or(stem);

                if check_stem2 && !(stem2 == "task" || stem2 == "tasks") {
                    continue;
                }

                break 'e true;
            }
            false
        };

        if !is_correct_ext {
            return None;
        }

        // check if it is executable
        // TODO error handling
        if !path.metadata().map(|m| m.permissions().mode() & 0o1 != 0).unwrap_or(false) {
            panic!("File is not executable: {:?}", path);
        }

        // TODO assert is file, extension and executable
        // TODO does this apply won windows? Or should we use "start <file>"?
        return Some(TaskfileSource::Executable(path.to_path_buf()));
    }

    // TODO error handling
    panic!("Failed to determine if file is a taskfile: {:?}", path);
}