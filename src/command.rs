use handlebars::Handlebars;
use serde::Serialize;



#[derive(Debug, Clone)]
pub enum Command {
    Shell(String),
}

impl Command {
    pub fn instantiate(
        &self,
        handlebars: &mut Handlebars,
        args: impl Serialize,
    ) -> Self {
        let Self::Shell(cmd) = self;
        let rendered = handlebars
            .render_template(cmd, &args)
            .expect("Failed to render command template");
        Command::Shell(rendered)
    }
}
