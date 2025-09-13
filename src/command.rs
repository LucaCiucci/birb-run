use handlebars::Handlebars;
use serde::Serialize;

use crate::task::BirbRenderContext;

#[derive(Debug, Clone)]
pub enum Command {
    Shell(String),
}

impl Command {
    pub fn instantiate(&self, handlebars: &mut Handlebars, args: impl Serialize, env: impl Serialize) -> Result<Self, CommandInstantiationError> {
        let Self::Shell(cmd) = self;
        let rendered = handlebars
            .render_template(cmd, &BirbRenderContext { args, env })?;
        Ok(Command::Shell(rendered))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandInstantiationError {
    #[error("Failed to render command template: {0}")]
    TemplateRenderError(#[from] handlebars::RenderError),
}
