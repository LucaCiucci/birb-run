use std::collections::{HashMap, HashSet, VecDeque};

use linked_hash_set::LinkedHashSet;

use crate::task::{InstantiatedTask, InstantiationError, ResolvedTaskInvocation, TaskInvocation, TaskRef, Taskfile, TaskfileId, Workspace};

pub mod naive;
pub mod topological_sort;

pub fn build_dependency_graph(
    workspace: &Workspace,
    current: &Taskfile,
    invocation: &TaskInvocation<TaskRef>,
) -> Result<(
    HashMap<ResolvedTaskInvocation, LinkedHashSet<ResolvedTaskInvocation>>,
    HashMap<ResolvedTaskInvocation, InstantiatedTask>,
), DependencyGraphConstructionError> {
    let mut queue: VecDeque<ResolvedTaskInvocation> = VecDeque::new();

    // Initialize the queue with the requested task invocation
    queue.push_back(
        workspace
            .resolve_invocation(current, invocation)
            .ok_or_else(|| DependencyGraphConstructionError::TaskfileInvocationResolutionError(
                current.id.clone(),
                invocation.clone(),
            ))?
            .0
    );

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

        let (tasks, task) = get_instantiation(workspace, &mut instantiations, &invocation)?;

        for dep in &task.body.deps.0 {
            let (dep, _task) = workspace
                .resolve_invocation(tasks, &dep.invocation)
                .ok_or_else(|| DependencyGraphConstructionError::TaskfileInvocationResolutionError(
                    tasks.id.clone(),
                    dep.invocation.clone(),
                ))?;
            node.insert(dep.clone());
            if !visited.contains(&dep) {
                queue.push_back(dep.clone());
            }
        }
    }

    Ok((graph, instantiations))
}

#[derive(Debug, thiserror::Error)]
pub enum DependencyGraphConstructionError {
    #[error("Failed to instantiate task: {0}")]
    InstantiationError(#[from] InstantiationError),
    #[error("Failed to resolve invocation {1:?} for taskfile {0}")]
    TaskfileInvocationResolutionError(TaskfileId, TaskInvocation<TaskRef>),
}

fn get_instantiation<'a>(
    workspace: &'a Workspace,
    instantiations: &'a mut HashMap<ResolvedTaskInvocation, InstantiatedTask>,
    invocation: &ResolvedTaskInvocation,
) -> Result<(&'a Taskfile, &'a InstantiatedTask), InstantiationError> {
    let (tasks, task) = workspace
        .resolve_invocation_task(&invocation)
        .expect(&format!("Task {} not found", invocation.r#ref.display_absolute()));

    let instantiation = {
        let e = instantiations.entry(invocation.clone());
        match e {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => e.insert(task.instantiate(&invocation.args)?),
        }
    };

    Ok((tasks, instantiation))
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
