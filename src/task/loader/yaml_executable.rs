use std::{any::Any, borrow::Cow, path::{Path, PathBuf}};

use crate::task::{yaml::YAML_DATA_EXTENSIONS, AbstractTaskfileSource, AbstractTaskfileSourceExt, Taskfile, TaskfileLoader, YamlLoadError};


/// Frontend for executables printing YAML to stdout
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YamlExecutableTaskfileLoader;

impl TaskfileLoader for YamlExecutableTaskfileLoader {
    fn find_taskfile(
        &self,
        path: &Path,
    ) -> Option<Box<dyn AbstractTaskfileSource>> {
        if path.is_file() {
            if has_correct_ext(path) && is_executable(path) {
                Some(Box::new(YamlTaskfileSource(path.to_path_buf())))
            } else {
                None
            }
        } else if path.is_dir() {
            for entry in std::fs::read_dir(path).ok()? {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && has_correct_ext_and_inner_stem(&path) == Some("tasks".to_string()) && is_executable(&path) {
                    return Some(Box::new(YamlTaskfileSource(path)));
                }
            }
            None
        } else {
            None
        }
    }

    fn load_taskfile(
        &self,
        source: Box<dyn AbstractTaskfileSource>,
    ) -> Result<crate::task::Taskfile, super::TaskfileLoadError> {
        let source: &YamlTaskfileSource = source.downcast_load()?;
        Ok(from_executable(&source.0)?)
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

// Example `tasks.yaml.sh`
fn has_correct_ext(path: &Path) -> bool {
    has_correct_ext_and_inner_stem(path).is_some()
}

fn has_correct_ext_and_inner_stem(path: &Path) -> Option<String> {
    // ignore the real extension
    let Some(stem) = path.file_stem() else {
        return None;
    };
    let stem = stem.to_string_lossy();

    for ext in YAML_DATA_EXTENSIONS {
        let e = format!(".{ext}");
        if stem.ends_with(&e) {
            let second_stem: &str = &stem[..stem.len() - e.len()];
            return Some(second_stem.to_string());
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        // On non-Unix systems, we can only check the file extension
        true
    }
}

fn from_executable(executable: impl AsRef<Path>) -> Result<Taskfile, YamlLoadError> {
    let executable = executable.as_ref();
    let working_dir = executable
        .parent()
        .ok_or(YamlLoadError::NoParentDirectory(executable.to_path_buf()))?;

    let output = std::process::Command::new(executable)
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| YamlLoadError::ExecutableRunError(executable.to_path_buf(), e))?;

    if !output.status.success() {
        return Err(YamlLoadError::ExecutableRunError(
            executable.to_path_buf(),
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Process exited with code {:?}", output.status.code()),
            ),
        ));
    }

    let source = String::from_utf8(output.stdout)
        .map_err(|e| YamlLoadError::ExecutableOutputNotUtf8(executable.to_path_buf(), e))?;

    Taskfile::from_yaml_source(&source, executable, working_dir)
}