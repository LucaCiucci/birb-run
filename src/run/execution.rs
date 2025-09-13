use std::{borrow::Borrow, collections::{BTreeMap, HashMap}, path::Path};

use colored::Colorize;
use pathdiff::diff_paths;
use serde_json::Value as Json;

use crate::{
    command::Command,
    run::{execution::{naive::NaiveExecutor, triggers::TaskTriggerChecker}, run_manager::TaskExecutionContext},
    task::{InstantiatedTask, OutputPath, ResolvedTaskInvocation, Taskfile},
};

pub mod naive;
pub mod triggers;
pub mod scheduler;

pub trait CommandExecutor {
    fn execute<C: Borrow<Command>>(
        &mut self,
        pwd: impl AsRef<Path>,
        env: &BTreeMap<String, Json>,
        commands: impl IntoIterator<Item = C>,
    ) -> anyhow::Result<()>; // TODO error type
}

#[derive(Debug, thiserror::Error)]
pub enum TaskExecutionError {
    #[error("Task not found for invocation {0:?}")]
    TaskNotFound(ResolvedTaskInvocation),
    #[error("Failed to remove {0}")]
    RemoveFileError(std::io::Error),
    #[error("Failed to build dependency graph: {0}")]
    ShouldRunCheckError(anyhow::Error),
    #[error("Output check failed: {0}")]
    OutputCheckError(anyhow::Error),
    #[error("Command execution failed: {0}")]
    CommandExecutorError(anyhow::Error), // TODO better error type
    #[error("Other")]
    Other(anyhow::Error), // TODO remove this
}

pub fn maybe_run_single_task<T: TaskTriggerChecker, C: TaskExecutionContext>(
    current: &Taskfile,
    tasks: &HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
    trigger_checker: &mut T,
    mut execution_context: C,
) -> Result<(), TaskExecutionError> {
    let task = tasks
        .get(&invocation)
        .ok_or(TaskExecutionError::TaskNotFound(invocation.clone()))?;

    let mut context = trigger_checker.new_task_context();

    log::trace!("Checking if task {:?} should run", invocation);
    let should_run = trigger_checker.should_run(task, &mut context)
        .map_err(|e| TaskExecutionError::ShouldRunCheckError(e.into()))?;
    log::trace!("Task {:?} should run: {}", invocation, should_run);

    if should_run {
        let mut env = current.env.clone();
        env.extend(task.body.env.clone());
        execution_context.run().execute(&task.body.workdir, &env, &task.body.steps).map_err(TaskExecutionError::CommandExecutorError)?;
    } else {
        execution_context.up_to_date();
    }

    trigger_checker.check_outputs(task, &mut context, should_run)
        .map_err(|e| TaskExecutionError::OutputCheckError(e.into()))?;

    Ok(())
}

pub fn clean_single_task(
    tasks: &Taskfile,
    instantiated_tasks: &HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
    output_handler: impl FnMut(&str),
) -> Result<(), TaskExecutionError> {
    let task = instantiated_tasks
        .get(&invocation)
        .ok_or(TaskExecutionError::TaskNotFound(invocation.clone()))?;

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    println!("    {} cleaning...", invocation.r#ref.display_relative(&cwd).to_string().bold().green());

    clean_instantiated_task(tasks, task, output_handler)?;

    Ok(())
}

pub fn clean_instantiated_task(
    tasks: &Taskfile,
    task: &InstantiatedTask,
    mut output_handler: impl FnMut(&str),
) -> Result<(), TaskExecutionError> {
    if let Some(clean_steps) = &task.body.clean {
        // HACK temporary solution
        let mut executor = NaiveExecutor {
            output_handler: &mut output_handler,
        };
        let mut env = tasks.env.clone();
        env.extend(task.body.env.clone());
        executor.execute(&task.body.workdir, &env, clean_steps).map_err(TaskExecutionError::Other)?;
    }

    for o in task.resolve_outputs() {
        let rel_path = diff_paths(
            o.as_ref(),
            std::env::current_dir().expect("Failed to get current directory"),
        )
        .unwrap_or(o.as_ref().to_path_buf());

        // ! if it fails, it we will not delete the file because we return early
        // this is ok since we want to avoid deleting files that are not in the
        // current directory as it would be dangerous
        let _rel_to_taskfile = o.as_ref().strip_prefix(&tasks.dir)
            .expect("Failed to compute relative path to taskfile directory");

        match o {
            OutputPath::File(path) => if Path::new(&path).exists() {
                std::fs::remove_file(path).map_err(TaskExecutionError::RemoveFileError)?;
                println!("{}\t{}", rel_path.display(), "REMOVED".magenta());
            } else {
                println!("{}\t{}", rel_path.display(), "NOT FOUND".bright_black());
            },
            OutputPath::Directory(path) => if Path::new(&path).exists() {
                std::fs::remove_dir_all(path).map_err(TaskExecutionError::RemoveFileError)?;
                println!("{}\t{}", rel_path.display(), "REMOVED".magenta());
            } else {
                println!("{}\t{}", rel_path.display(), "NOT FOUND".bright_black());
            },
        }
    }

    Ok(())
}