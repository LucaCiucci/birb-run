use std::{borrow::Borrow, collections::HashMap, path::Path};

use colored::Colorize;
use pathdiff::diff_paths;

use crate::{
    command::Command,
    run::execution::{naive::NaiveExecutor, triggers::TaskTriggerChecker},
    task::{InstantiatedTask, TaskInvocation},
};

pub mod naive;
pub mod triggers;

pub trait CommandExecutor {
    fn execute<C: Borrow<Command>>(
        &self,
        pwd: impl AsRef<Path>,
        commands: impl IntoIterator<Item = C>,
        output_handler: impl FnMut(&str),
    );
}

pub fn maybe_run_single_task(
    tasks: &HashMap<TaskInvocation, InstantiatedTask>,
    invocation: &TaskInvocation,
    trigger_checker: &mut impl TaskTriggerChecker,
    mut output_handler: impl FnMut(&str),
) -> bool {
    let task = tasks
        .get(&invocation)
        .expect("Task not found in the task list");

    let mut context = trigger_checker.new_task_context();

    let should_run = trigger_checker.should_run(task, &mut context);

    if should_run {
        let args = invocation
            .args
            .iter()
            .map(|(k, v)| format!("{}={}", k, &format!("{:.10}", v)))
            .collect::<Vec<_>>()
            .join(" ");
        output_handler(&format!("    {} {args}\trunning...", invocation.name.bold().green()));
        NaiveExecutor.execute(&task.body.workdir, &task.body.steps, output_handler);
    } else {
        output_handler(&format!("    {}\tup-to-date.", invocation.name.bold().cyan()));
    }

    trigger_checker.check_outputs(task, &mut context, should_run);

    return true;
}

pub fn clean_single_task(
    tasks: &HashMap<TaskInvocation, InstantiatedTask>,
    invocation: &TaskInvocation,
) {
    let task = tasks
        .get(&invocation)
        .expect("Task not found in the task list");
    clean_instantiated_task(task);
}

pub fn clean_instantiated_task(task: &InstantiatedTask) {
    for path in task.resolve_outputs() {
        let path: &Path = path.as_ref();
        let rel_path = diff_paths(
            path,
            std::env::current_dir().expect("Failed to get current directory"),
        )
        .expect("Failed to compute relative path");
        if path.exists() {
            std::fs::remove_file(path).expect("Failed to remove output file");
            println!("    {}\t{}", rel_path.display(), "REMOVED".magenta());
        } else {
            println!("    {}\t{}", rel_path.display(), "NOT FOUND".bright_black());
        }
    }
}