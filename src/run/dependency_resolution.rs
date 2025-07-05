use std::collections::{HashMap, HashSet, VecDeque};

use linked_hash_map::LinkedHashMap;
use linked_hash_set::LinkedHashSet;

use crate::task::{InstantiatedTask, Task, TaskInvocation};

pub mod naive;
pub mod topological_sort;

pub fn build_dependency_graph(
    tasks: &LinkedHashMap<String, Task>,
    request: &TaskInvocation,
) -> (
    HashMap<TaskInvocation, LinkedHashSet<TaskInvocation>>,
    HashMap<TaskInvocation, InstantiatedTask>,
) {
    let mut queue: VecDeque<TaskInvocation> = VecDeque::new();

    // Initialize the queue with the requested task invocation
    queue.push_back(request.clone());

    let mut visited = HashSet::new();

    let mut graph = HashMap::new();
    let mut instantiations = HashMap::new();

    while let Some(invocation) = queue.pop_front() {
        if visited.contains(&invocation) {
            continue;
        }
        visited.insert(invocation.clone());

        let node = graph
            .entry(invocation.clone())
            .or_insert_with(LinkedHashSet::new);

        let task = get_instantiation(tasks, &mut instantiations, &invocation);

        for dep in &task.body.deps.0 {
            let dep = &dep.invocation;
            node.insert(dep.clone());
            if !visited.contains(dep) {
                queue.push_back(dep.clone());
            }
        }
    }

    (graph, instantiations)
}

fn get_instantiation<'a>(
    tasks: &LinkedHashMap<String, Task>,
    instantiations: &'a mut HashMap<TaskInvocation, InstantiatedTask>,
    invocation: &TaskInvocation,
) -> &'a InstantiatedTask {
    let task = tasks
        .get(&invocation.name)
        .expect("Task not found in the task list");

    instantiations
        .entry(invocation.clone())
        .or_insert_with(|| task.instantiate(invocation.args.clone()).unwrap())
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DependencyResolutionError {
    #[error("Topological sort error: {0}")]
    TopologicalSortError(#[from] TopologicalSortError),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TopologicalSortError {
    #[error("Cycle detected in the dependency graph: {0:?}")]
    CycleDetected(Vec<TaskInvocation>),
}
