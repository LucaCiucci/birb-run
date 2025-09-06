use crate::{run::execution::CommandExecutor, task::ResolvedTaskInvocation};


pub mod default;
pub mod parallel;

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