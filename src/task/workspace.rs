use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use linked_hash_map::LinkedHashMap;

use crate::task::{yaml::YamlTaskfileLoader, yaml_executable::YamlExecutableTaskfileLoader, AbstractTaskfileSource, ResolvedTaskInvocation, Task, TaskInvocation, TaskRef, Taskfile, TaskfileId, TaskfileImportRef, TaskfileLoadError, TaskfileLoader};

#[derive(Debug, Clone)]
pub struct Workspace {
    frontends: LinkedHashMap<String, Arc<dyn TaskfileLoader>>,
    tasks: HashMap<TaskfileId, Taskfile>, // TODO maybe unnecessary, use ref if possible
}

impl Workspace {
    pub fn new() -> Self {
        let mut slf = Self {
            frontends: LinkedHashMap::new(),
            tasks: HashMap::new(),
        };

        slf.frontends.insert("yaml".into(), Arc::new(YamlTaskfileLoader));
        slf.frontends.insert("exe".into(), Arc::new(YamlExecutableTaskfileLoader));

        slf
    }

    pub fn from_main(path: impl Into<PathBuf>) -> Result<(Self, TaskfileId), WorkspaceLoadError> {
        let mut workspace = Self::new();
        let id = workspace.load_taskfile(path)?;
        Ok((workspace, id))
    }

    pub fn get<'a>(&'a self, id: &TaskfileId) -> Option<&'a Taskfile> {
        self.tasks.get(id)
    }

    pub fn get_id_from_path<'a>(&'a self, path: impl AsRef<Path>) -> Option<TaskfileId> {
        self.tasks.get(&TaskfileId::from_path(path.as_ref())).map(|t| t.id.clone())
    }

    /// Find the taskfile at a given path using the registered frontends.
    pub fn find_taskfile_source_at(&self, path: &Path) -> Vec<(&String, &Arc<dyn TaskfileLoader>, Box<dyn AbstractTaskfileSource>)> {
        let mut results = Vec::new();
        for (name, frontend) in &self.frontends {
            if let Some(source) = frontend.find_taskfile(path) {
                results.push((name, frontend, source));
            }
        }
        results
    }

    /// Find the taskfile at a given path using the registered frontends, searching parent directories.
    pub fn find_taskfile_source(&self, path: &Path) -> Vec<(&String, &Arc<dyn TaskfileLoader>, Box<dyn AbstractTaskfileSource>)> {
        let is_dir = path.is_dir();
        let paths = std::iter::once(path)
            .chain(std::iter::successors(path.parent(), |p| p.parent()).take_while(|_| is_dir));
        for path in paths {
            let result = self.find_taskfile_source_at(path);
            if !result.is_empty() {
                return result;
            }
        }
        Vec::new()
    }

    // TODO lazy load of imports?
    pub fn load_taskfile(&mut self, path: impl Into<PathBuf>) -> Result<TaskfileId, WorkspaceLoadError> {
        let path = path.into();
        //let source = Taskfile::find_taskfile(&path).ok_or(WorkspaceLoadError::TaskfileNotFound)?;
        let results = self.find_taskfile_source(&path);
        if results.len() > 1 {
            let list = results
                .iter()
                .map(|(name, _, source)| format!("- {} ({})\n", name, source.path().display()))
                .collect::<Vec<_>>()
                .join("");
            log::warn!("Multiple taskfile frontends found for {}:\n{list}", path.display());
        }
        let (frontend_name, frontend, source) = results.into_iter().next().ok_or(WorkspaceLoadError::TaskfileNotFound)?;
        let taskfile_path = source
            .path()
            .canonicalize()
            .map_err(|_| WorkspaceLoadError::Canonicalize(path.clone()))?;

        if let Some(id) = self.get_id_from_path(&taskfile_path) {
            return Ok(id);
        }

        let tasks = frontend.load_taskfile(source).map_err(WorkspaceLoadError::TaskfileLoadError)?;

        let mut imports = tasks.imports.clone();
        let id = TaskfileId::from_path(taskfile_path.clone());

        self.tasks.insert(tasks.id.clone(), tasks);

        for (_import_name, import) in &mut imports {
            // TODO it may be worth to possibly store Weak instead of using a TaskRef
            // that would need to be resolved later
            match import {
                TaskfileImportRef::Resolved(id) => assert!(self.tasks.contains_key(&id), "Resolved import not found in workspace"),
                TaskfileImportRef::Unresolved(import_path) => {
                    let imported = self.load_taskfile(import_path.as_path())?;
                    *import = TaskfileImportRef::Resolved(imported);
                }
            }
        }
        self
            .tasks
            .get_mut(&id)
            .expect("Failed to get taskfile that was just inserted")
            .imports = imports;

        Ok(id)
    }

    pub fn resolve_task<'a>(&'a self, current: &'a Taskfile, r#ref: &TaskRef) -> Option<(&'a Taskfile, &'a Task)> {
        match r#ref {
            TaskRef::Name(name) => Some((current, current.tasks.get(name)?)),
            TaskRef::Imported { from, name } => {
                let id = match current.imports.get(from)? {
                    TaskfileImportRef::Resolved(id) => id,
                    TaskfileImportRef::Unresolved(import_path) => {
                        panic!("Unresolved import path: {}", import_path.display())
                    }
                };
                let tasks = self.tasks.get(id)?;
                let task = tasks.tasks.get(name)?;
                Some((tasks, task))
            },
        }
    }

    // TODO unused?
    pub fn resolve_invocation<'a>(&'a self, current: &'a Taskfile, invocation: &TaskInvocation<TaskRef>) -> Option<(ResolvedTaskInvocation, &'a Task)> {
        let (tasks, task) = self.resolve_task(current, &invocation.r#ref)?;
        let resolved = invocation.as_resolved(tasks);
        Some((resolved, task))
    }

    pub fn resolve_invocation_task<'a>(&'a self, invocation: &ResolvedTaskInvocation) -> Option<(&'a Taskfile, &'a Task)> {
        let tasks = self.tasks.get(&invocation.r#ref.taskfile)?;
        let task = tasks.tasks.get(&invocation.r#ref.name)?;
        Some((tasks, task))
    }
}


#[derive(Debug, thiserror::Error)]
pub enum WorkspaceLoadError {
    #[error("Taskfile not found in the current directory or any parent directory")]
    TaskfileNotFound,
    #[error("Failed to canonicalize taskfile path: {0}")]
    Canonicalize(PathBuf),
    #[error("Failed to load taskfile")]
    TaskfileLoadError(#[from] TaskfileLoadError),
    //#[error("Failed to load taskfile from {0}: {1}")]
    //Yaml(PathBuf, YamlLoadError),
}