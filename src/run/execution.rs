use std::{borrow::Borrow, collections::HashMap, path::Path};

use colored::Colorize;
use pathdiff::diff_paths;

use crate::{
    command::Command,
    run::{execution::{naive::NaiveExecutor, triggers::TaskTriggerChecker}, TaskExecutionContext},
    task::{InstantiatedTask, ResolvedTaskInvocation, Taskfile},
};

pub mod naive;
pub mod triggers;

pub trait CommandExecutor {
    fn execute<C: Borrow<Command>>(
        &mut self,
        pwd: impl AsRef<Path>,
        commands: impl IntoIterator<Item = C>,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Task not found for invocation {0:?}")]
    TaskNotFound(ResolvedTaskInvocation),
    #[error("Failed to remove {0}")]
    RemoveFileError(std::io::Error),
}

pub fn maybe_run_single_task(
    tasks: &HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
    trigger_checker: &mut impl TaskTriggerChecker,
    mut execution_context: impl TaskExecutionContext,
) -> Result<(), ExecutionError> {
    let task = tasks
        .get(&invocation)
        .ok_or(ExecutionError::TaskNotFound(invocation.clone()))?;

    let mut context = trigger_checker.new_task_context();

    let should_run = trigger_checker.should_run(task, &mut context);

    if should_run {
        execution_context.run().execute(&task.body.workdir, &task.body.steps);
    } else {
        execution_context.up_to_date();
    }

    trigger_checker.check_outputs(task, &mut context, should_run);

    Ok(())
}

pub fn clean_single_task(
    tasks: &Taskfile,
    instantiated_tasks: &HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
    output_handler: impl FnMut(&str),
) -> Result<(), ExecutionError> {
    let task = instantiated_tasks
        .get(&invocation)
        .ok_or(ExecutionError::TaskNotFound(invocation.clone()))?;

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    println!("    {} cleaning...", invocation.r#ref.display_relative(&cwd).to_string().bold().green());

    clean_instantiated_task(tasks, task, output_handler)?;

    Ok(())
}

pub fn clean_instantiated_task(
    tasks: &Taskfile,
    task: &InstantiatedTask,
    mut output_handler: impl FnMut(&str),
) -> Result<(), ExecutionError> {
    if let Some(clean_steps) = &task.body.clean {
        // HACK temporary solution
        let mut executor = NaiveExecutor {
            output_handler: &mut output_handler,
        };
        executor.execute(&task.body.workdir, clean_steps);
    }

    for path in task.resolve_outputs() {
        let path: &Path = path.as_ref();
        let rel_path = diff_paths(
            path,
            std::env::current_dir().expect("Failed to get current directory"),
        )
        .unwrap_or(path.to_path_buf());

        // ! if it fails, it we will not delete the file because we return early
        // this is ok since we want to avoid deleting files that are not in the
        // current directory as it would be dangerous
        let _rel_to_taskfile = path.strip_prefix(&tasks.dir)
            .expect("Failed to compute relative path to taskfile directory");

        if path.exists() {
            std::fs::remove_file(path).map_err(ExecutionError::RemoveFileError)?;
            println!("{}\t{}", rel_path.display(), "REMOVED".magenta());
        } else {
            println!("{}\t{}", rel_path.display(), "NOT FOUND".bright_black());
        }
    }

    Ok(())
}