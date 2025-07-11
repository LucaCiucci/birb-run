use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::task::{ResolvedTaskInvocation, Task, TaskInvocation, TaskRef, Taskfile, TaskfileId, TaskfileImportRef};

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    tasks: HashMap<TaskfileId, Taskfile>, // TODO maybe unnecessary, use ref if possible
}

impl Workspace {
    pub fn from_main(path: impl Into<PathBuf>) -> (Self, TaskfileId) {
        let mut workspace = Self::default();
        let id = workspace.load_taskfile(path);
        (workspace, id)
    }

    pub fn get<'a>(&'a self, id: &TaskfileId) -> Option<&'a Taskfile> {
        self.tasks.get(id)
    }

    pub fn get_id_from_path<'a>(&'a self, path: impl AsRef<Path>) -> Option<TaskfileId> {
        self.tasks.get(&TaskfileId::from_path(path.as_ref())).map(|t| t.id.clone())
    }

    // TOOD lazy load of imports?
    pub fn load_taskfile(&mut self, path: impl Into<PathBuf>) -> TaskfileId {
        let path = path.into();
        let taskfile_path = Taskfile::find_taskfile(&path).canonicalize().unwrap();

        if let Some(id) = self.get_id_from_path(&taskfile_path) {
            return id;
        }

        let tasks = Taskfile::parse_yaml(&taskfile_path);
        let mut imports = tasks.imports.clone();
        let id = TaskfileId::from_path(taskfile_path.clone());

        self.tasks.insert(tasks.id.clone(), tasks);

        for (_import_name, import) in &mut imports {
            // TODO it may be worth to possibly store Weak instead of using a TaskRef
            // that would need to be resolved later
            match import {
                TaskfileImportRef::Resolved(id) => assert!(self.tasks.contains_key(&id), "Resolved import not found in workspace"),
                TaskfileImportRef::Unresolved(import_path) => {
                    let imported = self.load_taskfile(import_path.as_path());
                    *import = TaskfileImportRef::Resolved(imported);
                }
            }
        }
        self.tasks.get_mut(&id).unwrap().imports = imports;

        id
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