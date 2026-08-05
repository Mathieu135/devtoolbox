pub mod cli;

mod project;
mod rabbitmq;

use anyhow::Result;
use cli::{Cli, Command, Service};
use project::Project;

pub fn run(cli: Cli) -> Result<()> {
    let project = Project::discover(&cli.project)?;

    match cli.command {
        Command::Add {
            service: Service::Rabbitmq,
            dry_run,
            force,
        } => rabbitmq::add(&project, dry_run, force),
        Command::Up {
            service: Service::Rabbitmq,
        } => rabbitmq::up(&project),
        Command::Doctor {
            service: Service::Rabbitmq,
        } => rabbitmq::doctor(&project),
    }
}
