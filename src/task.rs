use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    path::{Path, PathBuf},
};

use handlebars::Handlebars;
use linked_hash_map::LinkedHashMap;
use serde::Serialize;
use serde_json::Value as Json;
use yaml_rust::{Yaml, YamlLoader};

use crate::command::Command;

mod from_yaml;

mod instantiation;

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
    pub fn no_args(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: BTreeMap::new(),
        }
    }

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

/// Represents a collection of tasks, usually loaded from a taskfile
pub struct Tasks {
    /// The path that generated this collection of tasks
    ///
    /// This is used to identify the taskfile for multi-taskfile projects
    pub path: PathBuf,

    /// The tasks in this collection, keyed by their names
    pub tasks: LinkedHashMap<String, Task>,
}

impl Tasks {
    pub fn new(
        path: impl Into<PathBuf>,
    ) -> Self {
        let path = path.into();
        assert!(path.is_file(), "Path must point to a file"); // TODO error handling
        Self {
            path,
            tasks: LinkedHashMap::new(),
        }
    }

    /// Finds the taskfile in the current directory or any parent directory
    pub fn find_taskfile(from: impl AsRef<Path>) -> PathBuf {
        let mut dir = from.as_ref().to_path_buf();
            loop {
                let path = dir.join("tasks.yaml");
                if path.is_file() {
                    break path;
                }

                // Try to find another "task.*" file that is executable, because taskfiles
                // can be be generated by other tools, for example tasks.json, task.sh, etc.
                //for entry in std::fs::read_dir(&dir).expect("Failed to read directory") {
                //    let entry = entry.expect("Failed to read directory entry");
                //    if entry.file_name().to_string_lossy().starts_with("task.")
                //        && entry.file_type().expect("Failed to get file type").is_file()
                //        && entry.metadata().expect("Failed to get metadata").permissions().mode() & 0o1 != 0
                //    {
                //        todo!()
                //    }
                //}

                if !dir.pop() {
                    panic!("No 'tasks.yaml' found in the current directory or any parent directory");
                }
            }
    }

    pub fn load_yaml_taskfile(taskfile: impl AsRef<Path>) -> Self {
        let taskfile = taskfile.as_ref();

        let mut this = Self::new(taskfile);

        let docs = YamlLoader::load_from_str(
            &std::fs::read_to_string(taskfile).expect("Failed to read 'tasks.yaml' file"),
        )
        .unwrap();

        for doc in docs {
            let doc = doc.as_hash().expect("Expected a YAML hash");

            let tasks = doc
                .get(&Yaml::String("tasks".into()))
                .expect("Expected 'tasks' key")
                .as_hash()
                .expect("Expected 'tasks' to be a hash");

            for (key, value) in tasks {
                let key = key.as_str().expect("Expected task key to be a string");
                let task = Task::from_yaml(&taskfile.parent().unwrap(), key, value);
                this.tasks.insert(key.to_string(), task.clone());
            }
        }

        this
    }

    pub fn invoke(&self, req: &TaskInvocation) {
        crate::run::run(&self.tasks, req);
    }

    pub fn clean(&self, req: &TaskInvocation, recursive: bool) {
        if recursive {
            crate::run::clean(&self.tasks, req);
        } else {
            crate::run::clean_only(&self.tasks, req);
        }
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
            ArgType::Select(options) => write!(f, "opt({})", options.join(", ")),
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
