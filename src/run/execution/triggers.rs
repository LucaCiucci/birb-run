use std::{collections::HashMap, fs::File, io::{BufReader, Read}, path::{Path, PathBuf}, time::SystemTime};

use anyhow::anyhow;
use sha2::{Digest, Sha256};

use crate::task::InstantiatedTask;

pub trait TaskTriggerChecker {
    type TaskContext;
    fn new_task_context(&mut self) -> Self::TaskContext;
    fn should_run(
        &mut self,
        task: &InstantiatedTask,
        context: &mut Self::TaskContext,
    ) -> bool;
    fn check_outputs(
        &mut self,
        task: &InstantiatedTask,
        context: &mut Self::TaskContext,
    );
}

#[derive(Debug, Default)]
pub struct NaiveTriggerChecker {
    not_changed: HashMap<PathBuf, bool>,
}

impl TaskTriggerChecker for NaiveTriggerChecker {
    type TaskContext = HashMap<PathBuf, Hash>;
    fn new_task_context(&mut self) -> Self::TaskContext {
        Default::default()
    }
    fn should_run(
        &mut self,
        task: &InstantiatedTask,
        context: &mut Self::TaskContext,
    ) -> bool {
        let output_hashes = context;

        let has_no_outputs = task.resolve_outputs().next().is_none();
        let has_no_command = task.body.steps.is_empty();

        // If a command does not have any output, we assume it should always run.
        if has_no_outputs {
            return true;
        }

        // If a command has no steps, it cannot be run.
        if has_no_command {
            return false;
        }

        sources_changed(task, output_hashes, &self.not_changed)
    }
    fn check_outputs(
        &mut self,
        task: &InstantiatedTask,
        context: &mut Self::TaskContext,
    ) {
        let output_hashes = context;

        let newest_source_timestamp = newest_input_timestamp(task, &self.not_changed)
            .expect("Failed to check input timestamps");

        for path in task.resolve_outputs() {
            let path: &Path = path.as_ref();
            if !path.exists() {
                panic!("Output file {} does not exist after running task '{}'", path.display(), task.name);
            }
            
            let metadata = std::fs::metadata(path)
                .expect("Failed to get metadata for output file");
            let output_timestamp = metadata.modified()
                .expect("Failed to get modified time for output file");

            if metadata.is_file() {
                // TODO we should also check if timestamp changed: if not, then
                // there is no point in reading it again and checking hash!
                let hash = hash_file(path);
                if let Some(prev_hash) = output_hashes.get(path) {
                    if let Some(previously_not_changed) = self.not_changed.get(path) {
                        // This is bad, it means that some other task invocation
                        // modified this same output file.
                        // It is not good to have more task to have the same output,
                        // we should issue a warning.
                        if *previously_not_changed {
                            // Anyway, if it did change in some other task, we certainly
                            // cannot un-change it so we do nothing.
                        } else {
                            self.not_changed.insert(path.into(), &hash == prev_hash);
                        }
                    } else {
                        // TODO avoid repetition
                        self.not_changed.insert(path.into(), &hash == prev_hash);
                    }
                } else {
                    // Again, we should issue a waring if already present.
                    self.not_changed.entry(path.into()).or_insert(false);
                }
            }

            if let Some(newest_source_timestamp) = &newest_source_timestamp {
                if &output_timestamp < newest_source_timestamp {
                    panic!("Output file {} is older than the newest source file for task '{}'", path.display(), task.name);
                }
            } else {
                // If there are no sources, we assume the output is valid.
                continue;
            }
        }
    }
}

type Hash = [u8; 32];

fn sources_changed(
    task: &InstantiatedTask,
    output_hashes: &mut HashMap<PathBuf, Hash>,
    not_changed: &HashMap<PathBuf, bool>,
) -> bool {
    let newest_source_timestamp = newest_input_timestamp(task, not_changed)
        .expect("Failed to check input timestamps");

    // check all output files against the source file timestamp
    let mut changed = false;
    for path in task.resolve_outputs() {
        let path: &Path = path.as_ref();
        if !path.exists() {
            // If the output file does not exist, we need certainly to run the task.
            changed = true;
            continue;
        }
        let metadata = std::fs::metadata(path)
            .expect("Failed to get metadata for output file");
        if let Some(newest_source_timestamp) = newest_source_timestamp {
            let output_timestamp = metadata.modified()
                .expect("Failed to get modified time for output file");
            if output_timestamp < newest_source_timestamp {
                // If the output file is older than the newest source file,
                // dependencies have changed.
                changed = true;
            }
        };

        if metadata.is_file() {
            output_hashes.insert(path.to_path_buf(), hash_file(path));
        }
    }

    changed || task.body.phony
}

// FIXME: this is called twice: first to check for inputs changes and then
// to verify the outputs. This is not efficient and should be optimized.
fn newest_input_timestamp(
    task: &InstantiatedTask,
    not_changed: &HashMap<PathBuf, bool>,
) -> anyhow::Result<Option<SystemTime>> {
    let mut newest_source_timestamp = None;

    for path in task.resolve_sources() {
        let path: &Path = path.as_ref();

        if let Some(not_changed) = not_changed.get(path) {
            if *not_changed {
                continue;
            }
        }

        if !path.exists() {
            return Err(anyhow!("Source file {path:?} does not exist"));
        }
        let metadata = std::fs::metadata(path)?;
        let timestamp = metadata.modified()?;

        if let Some(oldest) = newest_source_timestamp {
            if timestamp > oldest {
                newest_source_timestamp = Some(timestamp);
            }
        } else {
            newest_source_timestamp = Some(timestamp);
        }
    }

    Ok(newest_source_timestamp)
}

fn hash_file(path: impl AsRef<Path>) -> Hash {
    let mut file = BufReader::new(File::open(path).unwrap());
    let mut buf = [0u8; 512];
    let mut hasher = Sha256::new();
    loop {
        let n = file.read(&mut buf).unwrap();
        if n <= 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    let hash = hash.as_slice();
    hash.try_into().unwrap()
}