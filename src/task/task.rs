use std::{collections::HashMap, path::PathBuf};

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

    pub fn resolve_outputs(&self) -> impl Iterator<Item = PathBuf> {
        self.body.outputs.files.iter().map(|file| {
            let mut path = self.body.workdir.clone();
            path.push(file);
            path
        })
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
                outputs: Outputs { files: Vec::new() },
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
    pub files: Vec<String>,
}