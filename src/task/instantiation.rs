use std::collections::BTreeMap;

use handlebars::Handlebars;
use serde_json::Value as Json;

use crate::{
    task::{Dep, Deps, InstantiatedTask, Outputs, Task, TaskBody},
    utils::type_checking::{TypeCheckError, check_type},
};

impl Task {
    pub fn instantiate(
        &self,
        args: &BTreeMap<String, Json>,
    ) -> Result<InstantiatedTask, ArgumentsCheckError> {
        self.check_args(&args)?;

        let mut handlebars = Handlebars::new();

        Ok(InstantiatedTask {
            name: self.name.clone(),
            body: TaskBody {
                workdir: handlebars
                    .render_template(&self.body.workdir.to_string_lossy(), &args)
                    .unwrap()
                    .into(),
                phony: self.body.phony,
                outputs: Outputs {
                    files: self
                        .body
                        .outputs
                        .files
                        .iter()
                        .map(|file| handlebars.render_template(file, &args).unwrap())
                        .collect(),
                },
                sources: self
                    .body
                    .sources
                    .iter()
                    .map(|source| handlebars.render_template(source, &args).unwrap())
                    .collect(),
                deps: Deps(
                    self.body
                        .deps
                        .0
                        .iter()
                        .map(|dep| Dep {
                            invocation: dep.invocation.instantiate(&mut handlebars, &args),
                        })
                        .collect::<Vec<_>>(),
                ),
                steps: self
                    .body
                    .steps
                    .iter()
                    .map(|step| step.instantiate(&mut handlebars, &args))
                    .collect(),
                clean: self
                    .body
                    .clean
                    .as_ref()
                    .map(|clean_steps| {
                        clean_steps
                            .iter()
                            .map(|step| step.instantiate(&mut handlebars, &args))
                            .collect()
                    }),
            },
        })
    }

    pub fn check_args(&self, args: &BTreeMap<String, Json>) -> Result<(), ArgumentsCheckError> {
        for (key, _) in &self.params {
            if !args.contains_key(key) {
                return Err(ArgumentsCheckError::NotFound { key: key.clone() });
            }
        }

        for (key, value) in args.iter() {
            let param = self
                .params
                .get(key)
                .ok_or_else(|| ArgumentsCheckError::NotFound { key: key.clone() })?;
            check_type(&param.ty, value).map_err(|err| ArgumentsCheckError::TypeError {
                key: key.clone(),
                err,
            })?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum InstantiationError {
    #[error("Invalid arguments: {0}")]
    ArgsError(#[from] ArgumentsCheckError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ArgumentsCheckError {
    #[error("Missing argument '{key}'")]
    NotFound { key: String },
    #[error("Argument '{key}' has invalid type: {err}")]
    TypeError { key: String, err: TypeCheckError },
}
