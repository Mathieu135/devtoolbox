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

    /// Démarre le service local, attend sa disponibilité et teste un aller-retour.
    Up {
        #[arg(value_enum, default_value_t = Service::Rabbitmq)]
        service: Service,
    },

    /// Explique l'état du service et teste la connexion réellement utilisée.
    Doctor {
        #[arg(value_enum, default_value_t = Service::Rabbitmq)]
        service: Service,
    },
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Service {
    #[default]
    Rabbitmq,
}
