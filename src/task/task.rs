use std::{collections::HashMap, path::{Path, PathBuf}};

use handlebars::Handlebars;
use serde::Serialize;
use yaml_rust::Yaml;

use crate::{command::Command, task::{from_yaml::{self, InvalidTaskObject}, params::Param, TaskInvocation, TaskRef}};


#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub description: Option<String>,
    pub params: HashMap<String, Param>,
    pub body: TaskBody,
}

#[derive(Debug, Clone)]
pub struct InstantiatedTask {
    pub name: String,
    pub body: TaskBody,
}

impl InstantiatedTask {
    pub fn resolve_sources(&self) -> impl Iterator<Item = PathBuf> {
        self.body.sources.iter().map(|source| {
            let mut path = self.body.workdir.clone();
            path.push(source);
            path
        })
    }

    pub fn resolve_outputs(&self) -> impl Iterator<Item = OutputPath> {
        self.body.outputs.paths.iter().map(move |file| file.resolve(&self.body.workdir))
    }
}

#[derive(Debug, Clone)]
pub struct TaskBody {
    pub workdir: PathBuf,
    pub phony: bool,
    pub outputs: Outputs,
    pub sources: Vec<String>,
    pub deps: Deps,
    pub steps: Vec<Command>,
    pub clean: Option<Vec<Command>>,
}

impl Task {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            params: HashMap::new(),
            body: TaskBody {
                workdir: PathBuf::new(),
                phony: false,
                outputs: Outputs { paths: Vec::new() },
                sources: Default::default(),
                deps: Deps(Vec::new()),
                steps: Default::default(),
                clean: None,
            },
        }
    }

    pub fn from_yaml(workdir: impl Into<PathBuf>, name: &str, value: &Yaml) -> Result<Self, InvalidTaskObject> {
        from_yaml::parse_task(workdir, name, value)
    }
}

// TODO Input -> Vec<Dep>
#[derive(Debug, Clone)]
pub struct Deps(pub Vec<Dep>);

#[derive(Debug, Clone)]
pub struct Dep {
    pub invocation: TaskInvocation<TaskRef>,
}

#[derive(Debug, Clone)]
pub struct Outputs {
    pub paths: Vec<OutputPath>,
}

#[derive(Debug, Clone)]
pub enum OutputPath {
    File(String),
    Directory(String),
}

impl OutputPath {
    pub fn instantiate(&self, handlebars: &mut Handlebars, args: &impl Serialize) -> Result<Self, OutputPathInstantiationError> {
        match self {
            OutputPath::File(path) => Ok(OutputPath::File(handlebars.render_template(path, args)?)),
            OutputPath::Directory(path) => Ok(OutputPath::Directory(handlebars.render_template(path, args)?)),
        }
    }

    pub fn resolve(&self, workdir: &PathBuf) -> Self {
        let mut path = workdir.clone();
        match self {
            OutputPath::File(file) => {
                path.push(file);
                OutputPath::File(path.to_string_lossy().to_string())
            }
            OutputPath::Directory(dir) => {
                path.push(dir);
                OutputPath::Directory(path.to_string_lossy().to_string())
            }
        }
    }
}

impl AsRef<Path> for OutputPath {
    fn as_ref(&self) -> &Path {
        match self {
            OutputPath::File(path) | OutputPath::Directory(path) => path.as_ref(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OutputPathInstantiationError {
    #[error("Failed to render template: {0}")]
    TemplateRenderError(#[from] handlebars::RenderError),
}
