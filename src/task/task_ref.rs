use std::{fmt::Display, path::Path};

use handlebars::Handlebars;
use serde::Serialize;

use crate::task::{BirbRenderContext, TaskfileId};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskRef {
    Name(String),
    Imported {
        from: String,
        name: String,
    }
}

impl TaskRef {
    pub fn parse(r#ref: impl AsRef<str>) -> Self {
        let (first, second) = {
            let mut parts = r#ref.as_ref().splitn(2, ":");
            (
                parts.next().expect("Expected at least one part in task reference"),
                parts.next(),
            )
        };
        if let Some(second) = second {
            Self::Imported {
                from: first.into(),
                name: (second.to_string()),
            }
        } else {
            Self::Name(first.to_string())
        }
    }
    
    /// instantiates a task reference with the given arguments
    /// the `from` field is not templates
    pub fn instantiate(&self, handlebars: &mut Handlebars, args: &impl Serialize, env: &impl Serialize) -> TaskRef {
        let render_name = |name: &str| handlebars
            .render_template(name, &BirbRenderContext { args, env })
            .expect("Failed to render task name template");

        match self {
            TaskRef::Name(name) => TaskRef::Name(render_name(name)),
            TaskRef::Imported { from, name } => TaskRef::Imported {
                from: from.clone(),
                name: render_name(name),
            },
        }
    }
}

impl Display for TaskRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskRef::Name(name) => write!(f, "{}", name),
            TaskRef::Imported { from, name } => write!(f, "{}:{}", from, name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedRef {
    pub taskfile: TaskfileId,
    pub name: String,
}

impl ResolvedRef {
    pub fn display_absolute(&self) -> impl Display {
        ResolvedRefDisplayAbsolute(self)
    }

    pub fn display_relative<'a>(&'a self, path: &'a Path) -> impl Display {
        ResolvedRefDisplayRelative(self, path)
    }
}

struct ResolvedRefDisplayAbsolute<'a>(&'a ResolvedRef);

impl Display for ResolvedRefDisplayAbsolute<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0.taskfile, self.0.name)
    }
}

struct ResolvedRefDisplayRelative<'a>(&'a ResolvedRef, &'a Path);

impl Display for ResolvedRefDisplayRelative<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dir_display = self.0.taskfile.display_relative(self.1).to_string();
        if dir_display.is_empty() {
            self.0.name.fmt(f)
        } else {
            write!(f, "{}:{}", dir_display, self.0.name)
        }
    }
}