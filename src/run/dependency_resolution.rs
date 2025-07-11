use std::collections::{HashMap, HashSet, VecDeque};

use linked_hash_set::LinkedHashSet;

use crate::task::{InstantiatedTask, ResolvedTaskInvocation, TaskInvocation, TaskRef, Taskfile, Workspace};

pub mod naive;
pub mod topological_sort;

pub fn build_dependency_graph(
    workspace: &Workspace,
    current: &Taskfile,
    invocation: &TaskInvocation<TaskRef>,
) -> (
    HashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>>,
    HashMap<ResolvedTaskInvocation, InstantiatedTask>,
) {
    let mut queue: VecDeque<ResolvedTaskInvocation> = VecDeque::new();

    // Initialize the queue with the requested task invocation
    queue.push_back(workspace.resolve_invocation(current, invocation).unwrap().0);

    let mut visited = HashSet::new();

    let mut graph: HashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>> = HashMap::new();
    let mut instantiations = HashMap::new();

    while let Some(invocation) = queue.pop_front() {
        if visited.contains(&invocation) {
            continue;
        }
        visited.insert(invocation.clone());

        let node: &mut LinkedHashSet<ResolvedTaskInvocation> = graph
            .entry(invocation.clone())
            .or_insert_with(LinkedHashSet::new);

        let (tasks, task) = get_instantiation(workspace, &mut instantiations, &invocation);

        for dep in &task.body.deps.0 {
            let r = workspace.resolve_invocation(tasks, &dep.invocation);
            let dep = r.unwrap().0;
            node.insert(dep.clone());
            if !visited.contains(&dep) {
                queue.push_back(dep.clone());
            }
        }
    }

    (graph, instantiations)
}

fn get_instantiation<'a>(
    workspace: &'a Workspace,
    instantiations: &'a mut HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
) -> (&'a Taskfile, &'a InstantiatedTask) {
    let (tasks, task) = workspace
        .resolve_invocation_task(&invocation)
        .expect(&format!("Task {} not found", invocation.r#ref.display_absolute()));

    let instantiation = instantiations
        .entry(invocation.clone())
        .or_insert_with(|| task.instantiate(&invocation.args).unwrap());

    (tasks, instantiation)
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
    CycleDetected(Vec<ResolvedTaskInvocation>),
}
