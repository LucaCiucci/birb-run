use std::path::PathBuf;

use anyhow::anyhow;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    cli::CliRunOptions, run::{
        dependency_resolution::{build_dependency_graph, topological_sort::topological_sort, DependencyGraphConstructionError, TopologicalSortError},
        execution::{clean_instantiated_task, clean_single_task, maybe_run_single_task, naive::NaiveExecutor, triggers::NaiveTriggerChecker, CommandExecutor, ExecutionError},
    }, task::{ResolvedTaskInvocation, TaskInvocation, TaskRef, Taskfile, Workspace}
};

pub mod dependency_resolution;
pub mod execution;

pub trait RunManager {
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> anyhow::Result<impl RunExecution>;
}

pub trait RunExecution {
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> anyhow::Result<impl TaskExecutionContext + 'a>;
}

pub trait TaskExecutionContext {
    fn run(&mut self) -> impl CommandExecutor;
    fn up_to_date(&mut self);
    // TODO clean, maybe?
}

pub struct DefaultRunManager<'a>(pub &'a CliRunOptions); // TODO also use options while cleaning

impl RunManager for DefaultRunManager<'_> {
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> anyhow::Result<impl RunExecution> {
        let bar = ProgressBar::new(invocations.into_iter().count() as u64);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.green/white}] {pos:>7}/{len:7} {msg}")?
            .progress_chars("=>-"));
        Ok(DefaultRunExecution {
            bar,
            options: self.0,
        })
    }
}

struct DefaultRunExecution<'a> {
    bar: ProgressBar,
    options: &'a CliRunOptions,
}

impl Drop for DefaultRunExecution<'_> {
    fn drop(&mut self) {
        self.bar.finish_with_message("All tasks completed");
    }
}

impl RunExecution for DefaultRunExecution<'_> {
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> anyhow::Result<impl TaskExecutionContext + 'a> {
        self.bar.inc(1);
        let args = display_args(invocation);
        self.bar.set_message(format!("task: {} {args}", invocation.r#ref.display_relative(&std::env::current_dir().unwrap()).to_string().bold().green()));
        Ok(DefaultTaskExecutionContext {
            bar: &self.bar,
            invocation,
            cwd: std::env::current_dir().map_err(|e| anyhow!("Failed to get current directory: {e}"))?,
            options: &self.options,
        })
    }
}

struct DefaultTaskExecutionContext<'a> {
    bar: &'a ProgressBar,
    invocation: &'a ResolvedTaskInvocation,
    cwd: PathBuf,
    options: &'a CliRunOptions,
}

impl TaskExecutionContext for DefaultTaskExecutionContext<'_> {
    fn run(&mut self) -> impl CommandExecutor {
        let args = display_args(self.invocation);
        if !self.options.compact {
            self.bar.suspend(|| {
                println!("    {} {args}\trunning...", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().green());
            });
        }
        NaiveExecutor {
            output_handler: |output| {
                self.bar.suspend(|| println!("{output}"));
            },
        }
    }

    fn up_to_date(&mut self) {
        if !self.options.compact {
            let args = display_args(self.invocation);
            self.bar.suspend(|| {
                println!("    {} {args}\tup-to-date.", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().cyan())
            });
        }
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

fn display_args(invocation: &ResolvedTaskInvocation) -> String {
    invocation
        .args
        .iter()
        .map(|(k, v)| format!("{}={}", k, &format!("{:.10}", v)))
        .collect::<Vec<_>>()
        .join(" ")
}
