use linked_hash_map::LinkedHashMap;

use crate::{
    run::{
        dependency_resolution::{build_dependency_graph, topological_sort::topological_sort},
        execution::{clean_instantiated_task, clean_single_task, maybe_run_single_task, triggers::NaiveTriggerChecker},
    },
    task::{Task, TaskInvocation},
};

pub mod dependency_resolution;
pub mod execution;

pub fn run(tasks: &LinkedHashMap<String, Task>, req: &TaskInvocation) {
    let (deps_graph, instantiations) = build_dependency_graph(tasks, req);

    let sorted = topological_sort(&deps_graph).unwrap();

    let mut trigger_checker = NaiveTriggerChecker::default();
    for invocation in sorted.iter().rev() {
        maybe_run_single_task(&instantiations, invocation, &mut trigger_checker);
    }
}

pub fn clean(tasks: &LinkedHashMap<String, Task>, req: &TaskInvocation) {
    let (deps_graph, instatiations) = build_dependency_graph(tasks, req);

    let sorted = topological_sort(&deps_graph).unwrap();

    for invocation in sorted.iter() {
        clean_single_task(&instatiations, invocation);
    }
}

pub fn clean_only(tasks: &LinkedHashMap<String, Task>, req: &TaskInvocation) {
    let task = tasks
        .get(&req.name)
        .expect("Task not found in the task list")
        .instantiate(&req.args)
        .expect("Failed to instantiate task");

    clean_instantiated_task(&task);
}
