use std::collections::BTreeMap;

use handlebars::{Handlebars, HelperDef, RenderErrorReason};
use serde_json::Value as Json;

use crate::{
    command::CommandInstantiationError, task::{Deps, InstantiatedTask, OutputPathInstantiationError, Outputs, Task, TaskBody}, utils::type_checking::{check_type, TypeCheckError}
};

impl Task {
    pub fn instantiate(
        &self,
        args: &BTreeMap<String, Json>,
    ) -> Result<InstantiatedTask, InstantiationError> {
        self.check_args(&args)?;

        let mut handlebars = init_handlebars();

        Ok(InstantiatedTask {
            name: self.name.clone(),
            body: TaskBody {
                workdir: handlebars
                    .render_template(&self.body.workdir.to_string_lossy(), &args)?
                    .into(),
                phony: self.body.phony,
                outputs: Outputs {
                    paths: self
                        .body
                        .outputs
                        .paths
                        .iter()
                        .map(|file| file.instantiate(&mut handlebars, args))
                        .collect::<Result<_, _>>()?,
                },
                sources: self
                    .body
                    .sources
                    .iter()
                    .map(|source| handlebars.render_template(source, &args))
                    .collect::<Result<_, _>>()?,
                deps: Deps(
                    self.body
                        .deps
                        .0
                        .iter()
                        .map(|dep| dep.instantiate(&mut handlebars, &args))
                        .collect::<Vec<_>>(),
                ),
                steps: self
                    .body
                    .steps
                    .iter()
                    .map(|step| step.instantiate(&mut handlebars, &args))
                    .collect::<Result<_, _>>()
                    .map_err(InstantiationError::StepsInstantiationError)?,
                clean: self
                    .body
                    .clean
                    .as_ref()
                    .map(|clean_steps| {
                        clean_steps
                            .iter()
                            .map(|step| step.instantiate(&mut handlebars, &args))
                            .collect::<Result<_, _>>()
                            .map_err(InstantiationError::CleanStepsInstantiationError)
                    }).transpose()?,
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

#[derive(Debug, thiserror::Error)]
pub enum InstantiationError {
    #[error("Invalid arguments: {0}")]
    ArgsError(#[from] ArgumentsCheckError),
    #[error("Failed to render template: {0}")]
    TemplateRenderError(#[from] handlebars::RenderError),
    #[error("Failed to instantiate output path: {0}")]
    OutputPathInstantiationError(#[from] OutputPathInstantiationError),
    #[error("Failed to instantiate steps: {0}")]
    StepsInstantiationError(CommandInstantiationError),
    #[error("Failed to instantiate clean steps: {0}")]
    CleanStepsInstantiationError(CommandInstantiationError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ArgumentsCheckError {
    #[error("Missing argument '{key}'")]
    NotFound { key: String },
    #[error("Argument '{key}' has invalid type: {err}")]
    TypeError { key: String, err: TypeCheckError },
}


fn init_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    //handlebars.register_escape_fn(handlebars::no_escape);
    handlebars.register_helper("fmt_precision", Box::new(FmtPrecision));
    handlebars
}

struct FmtPrecision;

impl HelperDef for FmtPrecision {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &handlebars::Helper<'rc>,
        _r: &'reg Handlebars<'reg>,
        _ctx: &'rc handlebars::Context,
        _rc: &mut handlebars::RenderContext<'reg, 'rc>,
        out: &mut dyn handlebars::Output,
    ) -> handlebars::HelperResult {
        let param = h.param(0).ok_or_else(|| {
            RenderErrorReason::ParamNotFoundForIndex("number", 0)
        })?;
        let num = param.value().as_f64().ok_or_else(|| {
            RenderErrorReason::InvalidParamType("number")
        })?;
        
        let param = h.param(1).ok_or_else(|| {
            RenderErrorReason::ParamNotFoundForIndex("precision", 1)
        })?;

        let precision = param.value().as_u64().ok_or_else(|| {
            RenderErrorReason::InvalidParamType("precision")
        })? as usize;

        let formatted = format!("{:.*}", precision, num);
        out.write(&formatted)?;
        Ok(())
    }
}