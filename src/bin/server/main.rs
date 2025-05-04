// src/bin/hyperion_server.rs
use clap::Parser;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use hyperion::Hyperion;
use hyperion::server::{HyperionServer, ServerConfig};

#[derive(Parser)]
#[command(name = "hyperion-server")]
#[command(about = "Serveur HTTP pour Hyperion Database", long_about = None)]
struct Cli {
    /// Chemin vers la base de données
    #[arg(short, long)]
    db_path: PathBuf,

    /// Port d'écoute
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Adresse d'écoute
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialiser le logger
    env_logger::init();
    
    // Parser les arguments
    let args = Cli::parse();
    
    // Créer un runtime Tokio manuellement au lieu d'utiliser la macro
    let rt = Runtime::new()?;
    
    // Exécuter le programme principal de manière asynchrone
    rt.block_on(async_main(args))
}

async fn async_main(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Créer la configuration du serveur
    let config = ServerConfig {
        port: args.port,
        host: args.host.clone(),
    };
    
    // Ouvrir la base de données de manière asynchrone
    println!("Ouverture de la base de données: {:?}", args.db_path);
    let store = hyperion::storage::PersistentStore::open_async(&args.db_path).await?;
    let hyperion = Hyperion::from_store(Box::new(store));
    
    // Créer et démarrer le serveur
    let server = HyperionServer::new(hyperion, config);
    
    println!("Démarrage du serveur sur {}:{}", args.host, args.port);
    server.run().await;
    
    Ok(())
}