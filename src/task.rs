use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    path::PathBuf,
};

use handlebars::Handlebars;
use serde::Serialize;
use serde_json::Value as Json;
use yaml_rust::Yaml;

use crate::command::Command;

mod from_yaml;

mod instantiation;

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
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
}

impl Task {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: HashMap::new(),
            body: TaskBody {
                workdir: PathBuf::new(),
                phony: false,
                outputs: Outputs { files: Vec::new() },
                sources: Default::default(),
                deps: Deps(Vec::new()),
                steps: Default::default(),
            },
        }
    }

    pub fn from_yaml(workdir: impl Into<PathBuf>, name: &str, value: &Yaml) -> Self {
        from_yaml::parse_task(workdir, name, value)
    }
}

// TODO Input -> Vec<Dep>
#[derive(Debug, Clone)]
pub struct Deps(pub Vec<Dep>);

#[derive(Debug, Clone)]
pub struct Dep {
    pub invocation: TaskInvocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskInvocation {
    pub name: String,
    pub args: BTreeMap<String, Json>,
}

impl TaskInvocation {
    pub fn instantiate(&self, handlebars: &mut Handlebars, args: &impl Serialize) -> Self {
        Self {
            name: handlebars
                .render_template(&self.name, args)
                .expect("Failed to render task name template"),
            args: self
                .args
                .iter()
                .map(|(k, v)| {
                    let rendered_value = instantiate_json_value(handlebars, v, args);
                    (k.clone(), rendered_value)
                })
                .collect(),
        }
    }
}

pub fn instantiate_json_value(
    handlebars: &mut Handlebars,
    value: &Json,
    args: &impl Serialize,
) -> Json {
    match value {
        Json::String(s) => {
            let rendered = handlebars
                .render_template(s, args)
                .expect("Failed to render string template");
            // reparse it, this is stupid and we should use a better way to understand if the rendered string expanded to something
            // like a number. An Idea could be to check if the template is just "{{}}"
            serde_json::from_str(&rendered).unwrap_or_else(|_| Json::String(rendered))
        }
        Json::Array(arr) => Json::Array(
            arr.iter()
                .map(|v| instantiate_json_value(handlebars, v, args))
                .collect(),
        ),
        Json::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                new_obj.insert(k.clone(), instantiate_json_value(handlebars, v, args));
            }
            Json::Object(new_obj)
        }
        _ => value.clone(),
    }
}

#[derive(Debug, Clone)]
pub enum ArgType {
    String,
    Select(Vec<String>),
    Number,
    Boolean,
    Path,
    Array(Box<ArgType>),
}

impl Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::String => write!(f, "string"),
            ArgType::Select(options) => write!(f, "select({})", options.join(", ")),
            ArgType::Number => write!(f, "number"),
            ArgType::Boolean => write!(f, "boolean"),
            ArgType::Path => write!(f, "path"),
            ArgType::Array(inner_type) => write!(f, "array<{}>", inner_type),
        }
    }
}

impl ArgType {
    pub fn validate(&self, value: &Json) -> bool {
        match self {
            ArgType::String => value.is_string(),
            ArgType::Select(options) => {
                if let Some(s) = value.as_str() {
                    options.contains(&s.to_string())
                } else {
                    false
                }
            }
            ArgType::Number => value.is_number(),
            ArgType::Boolean => value.is_boolean(),
            ArgType::Path => value.is_string(), // Assuming path is a string
            ArgType::Array(inner_type) => {
                if let Some(arr) = value.as_array() {
                    arr.iter().all(|v| inner_type.validate(v))
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: ArgType,
    pub default: Option<Json>,
}

impl Param {
    pub fn validate_default(&self) -> bool {
        if let Some(default) = &self.default {
            self.ty.validate(default)
        } else {
            true // If no default is provided, we consider it valid
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outputs {
    pub files: Vec<String>,
}
