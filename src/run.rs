use std::path::PathBuf;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    run::{
        dependency_resolution::{build_dependency_graph, topological_sort::topological_sort, DependencyGraphConstructionError, TopologicalSortError},
        execution::{clean_instantiated_task, clean_single_task, maybe_run_single_task, naive::NaiveExecutor, triggers::NaiveTriggerChecker, CommandExecutor, ExecutionError},
    },
    task::{ResolvedTaskInvocation, TaskInvocation, TaskRef, Taskfile, Workspace},
};

pub mod dependency_resolution;
pub mod execution;

pub trait RunManager {
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> impl RunExecution;
}

pub trait RunExecution {
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> impl TaskExecutionContext + 'a;
}

pub trait TaskExecutionContext {
    fn run(&mut self) -> impl CommandExecutor;
    fn up_to_date(&mut self);
    // TODO clean, maybe?
}

pub struct DefaultRunManager;

impl RunManager for DefaultRunManager {
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> impl RunExecution {
        let bar = ProgressBar::new(invocations.into_iter().count() as u64);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.green/white}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"));
        DefaultRunExecution {
            bar,
        }
    }
}

struct DefaultRunExecution {
    bar: ProgressBar,
}

impl Drop for DefaultRunExecution {
    fn drop(&mut self) {
        self.bar.finish_with_message("All tasks completed");
    }
}

impl RunExecution for DefaultRunExecution {
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> impl TaskExecutionContext + 'a {
        self.bar.inc(1);
        self.bar.set_message(format!("task: {}", invocation.r#ref.name));
        DefaultTaskExecutionContext {
            bar: &self.bar,
            invocation,
            cwd: std::env::current_dir().expect("Failed to get current directory"),
        }
    }
}

struct DefaultTaskExecutionContext<'a> {
    bar: &'a ProgressBar,
    invocation: &'a ResolvedTaskInvocation,
    cwd: PathBuf
}

impl TaskExecutionContext for DefaultTaskExecutionContext<'_> {
    fn run(&mut self) -> impl CommandExecutor {
        let args = self.invocation
            .args
            .iter()
            .map(|(k, v)| format!("{}={}", k, &format!("{:.10}", v)))
            .collect::<Vec<_>>()
            .join(" ");
        self.bar.suspend(|| {
            println!("    {} {args}\trunning...", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().green());
        });
        NaiveExecutor {
            output_handler: |output| {
                self.bar.suspend(|| println!("{output}"));
            },
        }
    }

    fn up_to_date(&mut self) {
        self.bar.suspend(|| {
            println!("    {}\tup-to-date.", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().cyan())
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("Failed to build dependency graph: {0}")]
    DependencyGraphConstructionError(#[from] DependencyGraphConstructionError),
    #[error("Failed to build dependency graph: {0}")]
    ExecutionError(#[from] ExecutionError),
    #[error("Topological sort error: {0}")]
    TopologicalSortError(#[from] TopologicalSortError),
    #[error("Task not found: {0}")]
    TaskNotFound(TaskRef),
    #[error("Failed to instantiate task: {0}")]
    InstantiationError(#[from] crate::task::InstantiationError),
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
    let execution = run_manager.begin(sorted.iter().rev());
    for invocation in sorted.iter().rev() {
        maybe_run_single_task(
            &instantiations,
            invocation,
            &mut trigger_checker,
            execution.enter_task(invocation),
        )?;
    }
    Ok(())
}

pub fn clean(
    workspace: &Workspace,
    current: &Taskfile,
    req: &TaskInvocation<TaskRef>,
) -> Result<(), RunError> {
    let (deps_graph, instatiations) = build_dependency_graph(workspace, current, req)?;

    let sorted = topological_sort(&deps_graph)?;

    for invocation in sorted.iter() {
        clean_single_task(current, &instatiations, invocation, |output| {
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
