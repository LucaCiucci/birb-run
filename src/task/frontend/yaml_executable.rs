use std::{any::Any, borrow::Cow, path::{Path, PathBuf}};

use crate::task::{yaml::YAML_DATA_EXTENSIONS, AbstractTaskfileSource, TaskfileFrontend};


/// Frontend for executables printing YAML to stdout
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YamlExecutableFrontend;

impl TaskfileFrontend for YamlExecutableFrontend {
    fn find_taskfile_in_dir(
        &self,
        path: &Path,
    ) -> Option<Box<dyn AbstractTaskfileSource>> {
        /// Example `tasks.yaml.sh`
        fn has_correct_ext(path: &Path) -> bool {
            // ignore the real extension
            let Some(file_name) = path.file_name() else {
                return false;
            };
            let file_name = file_name.to_string_lossy();

            for ext in YAML_DATA_EXTENSIONS {
                if file_name.ends_with(&format!(".{ext}")) {
                    return true;
                }
            }

            false
        }

        
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