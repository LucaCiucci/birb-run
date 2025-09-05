use std::sync::{Arc, Mutex};

pub mod default_run_manager;
pub mod parallel_run_manager;

use crate::{
    run::{
        dependency_resolution::{build_dependency_graph, topological_sort::topological_sort, DependencyGraphConstructionError, TopologicalSortError},
        execution::{clean_instantiated_task, clean_single_task, maybe_run_single_task, scheduler::execute_tasks_concurrently, triggers::NaiveTriggerChecker, CommandExecutor, TaskExecutionError},
    }, task::{ResolvedTaskInvocation, TaskInvocation, TaskRef, Taskfile, Workspace}
};

pub mod dependency_resolution;
pub mod execution;

pub trait RunManager: Send + Sync {
    type RunExecution: RunExecution;
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> anyhow::Result<Self::RunExecution>;
}

pub trait RunExecution: Send + Sync {
    type TaskExecutionContext<'a>: TaskExecutionContext where Self: 'a;
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> anyhow::Result<Self::TaskExecutionContext<'a>>;
}

pub trait TaskExecutionContext: Send + Sync {
    fn run(&mut self) -> impl CommandExecutor;
    fn up_to_date(&mut self);
    // TODO clean, maybe?
}



#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("Failed to build dependency graph: {0}")]
    DependencyGraphConstructionError(#[from] DependencyGraphConstructionError),
    #[error("Failed to build dependency graph: {0}")]
    ExecutionError(#[from] TaskExecutionError),
    #[error("Topological sort error: {0}")]
    TopologicalSortError(#[from] TopologicalSortError),
    #[error("Task not found: {0}")]
    TaskNotFound(TaskRef),
    #[error("Failed to instantiate task: {0}")]
    InstantiationError(#[from] crate::task::InstantiationError),
    #[error("Manager failed to begin task: {0}")]
    BeginTaskError(anyhow::Error),
    #[error("Manager run execution failed enter task: {0}")]
    EnterTaskError(anyhow::Error),
}

pub fn run(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
    run_manager: impl RunManager,
) -> Result<(), RunError> {
    let (deps_graph, instantiations) = build_dependency_graph(workspace, current, req)?;

    let sorted = topological_sort(&deps_graph)?;

    let mut trigger_checker = NaiveTriggerChecker::default();
    let execution = run_manager.begin(sorted.iter().rev()).map_err(RunError::BeginTaskError)?;
    for invocation in sorted.iter().rev() {
        maybe_run_single_task(
            &instantiations,
            invocation,
            &mut trigger_checker,
            execution.enter_task(invocation).map_err(RunError::EnterTaskError)?,
        )?;
    }
    Ok(())
}

pub async fn run_parallel(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
    run_manager: impl RunManager + 'static,
    max_concurrency: usize,
) -> Result<(), RunError> {
    let (deps_graph, instantiations) = build_dependency_graph(workspace, current, req)?;

    let sorted = topological_sort(&deps_graph)?;

    let trigger_checker = Arc::new(Mutex::new(NaiveTriggerChecker::default()));

    let execution = run_manager.begin(sorted.iter().rev()).map_err(RunError::BeginTaskError)?;
    let execution = Arc::new(execution);

    let instantiations = Arc::new(instantiations);

    // TODO concurrency as a parameter
    let r = execute_tasks_concurrently(
        max_concurrency, // TODO maybe physical instead?
        sorted.iter().rev().cloned(), // FIXME stupid af
        deps_graph,
        move|invocation| {
            let instantiations = instantiations.clone();
            let invocation  = invocation.clone(); // TODO avoid clone
            let mut trigger_checker = trigger_checker.clone();
            let execution = execution.clone();
            async move {
                let r = tokio::task::spawn_blocking(move || -> Result<(), RunError> {
                    let cx = execution.enter_task(&invocation).map_err(RunError::EnterTaskError);
                    let r = maybe_run_single_task(
                        &*instantiations,
                        &invocation,
                        &mut trigger_checker,
                        cx?,
                    )?;
                    Ok(r)
                }).await.unwrap();
                Ok(r?)
            }
        },
    ).await;

    r.map_err(|e| RunError::ExecutionError(TaskExecutionError::Other(e)))
}

pub fn clean(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) -> Result<(), RunError> {
    let (deps_graph, instantiations) = build_dependency_graph(workspace, current, req)?;

    let sorted = topological_sort(&deps_graph)?;

    for invocation in sorted.iter() {
        clean_single_task(current, &instantiations, invocation, |output| {
            println!("{}", output);
        })?;
    }
    Ok(())
}

pub fn clean_only(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) -> Result<(), RunError> {
    let task = workspace.resolve_task(current, &req.r#ref)
        .ok_or_else(|| RunError::TaskNotFound(req.r#ref.clone()))?
        .1
        .instantiate(&req.args)?; // TODO error handling

    clean_instantiated_task(current, &task, |output| {
        println!("{}", output);
    })?;
    Ok(())
}

fn display_args(invocation: &ResolvedTaskInvocation) -> String {
    invocation
        .args
        .iter()
        .map(|(k, v)| format!("{}={}", k, &format!("{:.10}", v)))
        .collect::<Vec<_>>()
        .join(" ")
}
