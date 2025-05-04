// src/bin/hyperion_cli/app.rs (modifié)
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::commands;
use crate::context::Context;
use crate::formatters::OutputFormat;
use crate::repl::Repl;

#[derive(Parser)]
#[command(name = "hyperion")]
#[command(about = "CLI pour Hyperion Database", long_about = None)]
struct Cli {
    /// Niveau de verbosité
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Format de sortie (text, json, table)
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Mode interactif (REPL)
    #[arg(short, long)]
    interactive: bool,

    /// URL du serveur Hyperion
    #[arg(short, long)]
    server: Option<String>,

    /// Commande à exécuter
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Se connecter à un serveur Hyperion
    Connect {
        /// URL du serveur
        url: String,
    },
    
    /// Exécuter une requête HyperionQL
    Query {
        /// Requête à exécuter
        query: String,
    },
    
    /// Lister les chemins dans la base de données
    List {
        /// Préfixe pour filtrer les chemins (optionnel)
        #[arg(short, long)]
        prefix: Option<String>,
    },
}

/// Exécute l'application CLI
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    
    // Créer un contexte
    let mut context = Context::new(cli.verbose, cli.format)?;
    
    // Si une URL de serveur est fournie, se connecter
    if let Some(server) = cli.server {
        commands::connect::execute(&mut context, &server)?;
    }
    
    // Exécuter la commande spécifiée ou entrer en mode interactif
    match (cli.command, cli.interactive) {
        (Some(Commands::Connect { url }), _) => {
            commands::connect::execute(&mut context, &url)?;
            println!("Connecté au serveur: {}", url);
        },
        (Some(Commands::Query { query }), _) => {
            commands::query::execute(&mut context, &query)?;
        },
        (Some(Commands::List { prefix }), _) => {
            commands::list::execute(&mut context, prefix.as_deref())?;
        },
        (None, true) | (None, _) if context.is_connected() => {
            // Mode interactif
            let mut repl = Repl::new(context);
            repl.run()?;
        },
        (None, _) => {
            println!("Erreur : Aucune commande spécifiée et non connecté à un serveur.");
            println!("Utilisez --help pour voir les options disponibles.");
        }
    }
    
    Ok(())
}