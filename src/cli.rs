use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "toolbox",
    version,
    about = "Ajoute, démarre et diagnostique les dépendances d'un projet"
)]
pub struct Cli {
    /// Projet ciblé. Un parent contenant Cargo.toml est détecté automatiquement.
    #[arg(long, global = true, default_value = ".")]
    pub project: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Génère le câblage local et un client de vérification réversible.
    Add {
        #[arg(value_enum)]
        service: Service,

        /// Affiche le plan sans écrire de fichier.
        #[arg(long)]
        dry_run: bool,

        /// Remplace les fichiers générés qui ont été modifiés manuellement.
        #[arg(long)]
        force: bool,
    },

    /// Démarre un service, ou tous les services installés si aucun n'est précisé.
    Up {
        #[arg(value_enum)]
        service: Option<Service>,
    },

    /// Diagnostique un service, ou tous les services installés.
    Doctor {
        #[arg(value_enum)]
        service: Option<Service>,
    },

    /// Arrête un service local, ou tous les services installés, sans supprimer les volumes.
    Down {
        #[arg(value_enum)]
        service: Option<Service>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Service {
    Rabbitmq,
    Redis,
}

impl Service {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rabbitmq => "RabbitMQ",
            Self::Redis => "Redis",
        }
    }
}
