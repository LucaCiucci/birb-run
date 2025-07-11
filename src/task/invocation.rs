use std::{collections::BTreeMap};
use handlebars::Handlebars;
use serde::Serialize;
use serde_json::Value as Json;

use crate::task::{ResolvedRef, TaskRef, Taskfile};

pub type ResolvedTaskInvocation = TaskInvocation<ResolvedRef>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskInvocation<Ref> {
    pub r#ref: Ref,
    pub args: BTreeMap<String, Json>,
}

impl<Ref> TaskInvocation<Ref> {
    pub fn no_args(r#ref: Ref) -> Self {
        Self {
            r#ref,
            args: BTreeMap::new(),
        }
    }
}

impl TaskInvocation<TaskRef> {
    pub fn instantiate(&self, handlebars: &mut Handlebars, args: &impl Serialize) -> Self {
        Self {
            r#ref: self.r#ref.instantiate(handlebars, args),
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

    pub fn as_resolved(&self, tasks: &Taskfile) -> TaskInvocation<ResolvedRef> {
        TaskInvocation {
            r#ref: ResolvedRef {
                taskfile: tasks.id.clone(),
                name: match &self.r#ref {
                    TaskRef::Name(name) => name.clone(),
                    TaskRef::Imported { from: _, name } => name.clone(),
                }
            },
            args: self.args.clone(),
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