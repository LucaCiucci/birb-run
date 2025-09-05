use clap::{builder::{EnumValueParser, TypedValueParser, ValueParserFactory}, ValueEnum};

use crate::cli::value_parser::CustomValueParser;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThreadsConfig {
    Named(NamedThreadConfig),
    Num(u64),
}

impl ThreadsConfig {
    pub fn get_num_threads(self) -> usize {
        match self {
            ThreadsConfig::Named(named) => match named {
                NamedThreadConfig::Cpu => num_cpus::get(),
                NamedThreadConfig::Physical => num_cpus::get_physical(),
            },
            ThreadsConfig::Num(n) => n as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(ValueEnum)]
pub enum NamedThreadConfig {
    /// Use the number of logical CPUs available.
    Cpu,
    /// Use the number of physical CPUs available.
    Physical,
}

impl ValueParserFactory for ThreadsConfig {
    type Parser = CustomValueParser<ThreadsConfig>;

    fn value_parser() -> Self::Parser {
        CustomValueParser::new()
    }
}

impl TypedValueParser for CustomValueParser<ThreadsConfig> {
    type Value = ThreadsConfig;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let e = EnumValueParser::<NamedThreadConfig>::new();
        let r = e.parse_ref(cmd, arg, value);
        match r {
            Ok(r) => Ok(ThreadsConfig::Named(r)),
            Err(enum_error) => {
                let p = clap::value_parser!(u64).range(1..);
                let r = p.parse_ref(cmd, arg, value);
                match r {
                    Ok(n) => Ok(ThreadsConfig::Num(n)),
                    Err(num_error) => Err(clap::Error::raw(
                        clap::error::ErrorKind::InvalidValue,
                        format!(
                            "Invalid thread configuration: not a named config ({enum_error}) or a number ({num_error})"
                        ),
                    )
                    .with_cmd(cmd))
                }
            }
        }
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        let e = EnumValueParser::<NamedThreadConfig>::new();
        let it = e.possible_values()?
            .collect::<Vec<_>>()
            .into_iter()
            .chain(std::iter::once(clap::builder::PossibleValue::new("<number>").help("A specific number of threads")));
        Some(Box::new(it))
    }
}