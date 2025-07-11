use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    run::{
        dependency_resolution::{build_dependency_graph, topological_sort::topological_sort},
        execution::{clean_instantiated_task, clean_single_task, maybe_run_single_task, triggers::NaiveTriggerChecker},
    },
    task::{TaskInvocation, TaskRef, Taskfile, Workspace},
};

pub mod dependency_resolution;
pub mod execution;

pub fn run(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) {
    let (deps_graph, instantiations) = build_dependency_graph(workspace, current, req);

    let sorted = topological_sort(&deps_graph).unwrap();

    let mut trigger_checker = NaiveTriggerChecker::default();
    let bar = ProgressBar::new(sorted.len() as u64);
    bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.green/white}] {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("=>-"));
    for invocation in sorted.iter().rev() {
        bar.set_message(format!("Running task: {}", invocation.r#ref.name));
        bar.inc(1);
        maybe_run_single_task(
            &instantiations,
            invocation,
            &mut trigger_checker,
            |output| bar.suspend(|| println!("{output}")),
        );
    }
    bar.finish_with_message("All tasks completed");
}

pub fn clean(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) {
    let (deps_graph, instatiations) = build_dependency_graph(workspace, current, req);

    let sorted = topological_sort(&deps_graph).unwrap();

    for invocation in sorted.iter() {
        clean_single_task(current, &instatiations, invocation, |output| {
            println!("{}", output);
        });
    }
}

pub fn clean_only(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) {
    let task = workspace.resolve_task(current, &req.r#ref)
        .expect("Task not found in the task list").1
        .instantiate(&req.args)
        .expect("Failed to instantiate task");

    clean_instantiated_task(current, &task, |output| {
        println!("{}", output);
    });
}
