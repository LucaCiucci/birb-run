use birb_task::cli::Cli;
use clap::Parser;


fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Cli::parse();

    birb_task::cli::main(&args)
}

