use birb_run::cli::Cli;
use clap::Parser;


fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Cli::parse();

    birb_run::cli::main(&args)
}

