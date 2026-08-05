pub mod cli;

mod project;
mod rabbitmq;
mod redis;

use anyhow::{Result, bail};
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
        Command::Add {
            service: Service::Redis,
            dry_run,
            force,
        } => redis::add(&project, dry_run, force),
        Command::Up { service } => for_services(&project, service, |service| match service {
            Service::Rabbitmq => rabbitmq::up(&project),
            Service::Redis => redis::up(&project),
        }),
        Command::Doctor { service } => for_services(&project, service, |service| match service {
            Service::Rabbitmq => rabbitmq::doctor(&project),
            Service::Redis => redis::doctor(&project),
        }),
        Command::Down { service } => for_services(&project, service, |service| match service {
            Service::Rabbitmq => rabbitmq::down(&project),
            Service::Redis => redis::down(&project),
        }),
    }
}

fn for_services<F>(project: &Project, selected: Option<Service>, mut operation: F) -> Result<()>
where
    F: FnMut(Service) -> Result<()>,
{
    let services = match selected {
        Some(service) => vec![service],
        None => {
            let mut installed = Vec::new();
            if rabbitmq::installed(project) {
                installed.push(Service::Rabbitmq);
            }
            if redis::installed(project) {
                installed.push(Service::Redis);
            }
            if installed.is_empty() {
                bail!(
                    "aucun service installé ; exécutez toolbox add rabbitmq ou toolbox add redis"
                );
            }
            installed
        }
    };

    for service in services {
        println!("\n== {} ==", service.name());
        operation(service)?;
    }
    Ok(())
}
