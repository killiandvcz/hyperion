mod history;

use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use crate::context::Context;
use crate::commands;
use self::history::History;

/// Mode interactif (REPL)
pub struct Repl {
    /// Contexte d'exécution
    context: Context,
    
    /// Éditeur de ligne
    editor: DefaultEditor,
    
    /// Historique personnalisé
    history: History,
}

impl Repl {
    /// Crée un nouveau REPL
    pub fn new(context: Context) -> Self {
        // Créer l'éditeur
        let mut editor = DefaultEditor::new().expect("Impossible de créer l'éditeur");
        
        // Créer l'historique
        let mut history = History::new("hyperion_history.txt", 1000);
        
        // Charger l'historique s'il existe
        let _ = history.load();
        
        // Charger l'historique dans l'éditeur
        for cmd in history.get_all() {
            let _ = editor.add_history_entry(cmd);
        }
        
        Repl {
            context,
            editor,
            history,
        }
    }
    
    /// Exécute le REPL
    pub fn run(&mut self) -> Result<()> {
        println!("{}", self.context.formatter().format_info("Hyperion CLI - Mode interactif"));
        println!("{}", self.context.formatter().format_info("Tapez .help pour l'aide ou .exit pour quitter"));
        
        loop {
            // Afficher le prompt
            let prompt = if self.context.is_connected() {
                "hyperion> "
            } else {
                "hyperion (déconnecté)> "
            };
            
            // Lire l'entrée utilisateur
            let readline = self.editor.readline(prompt);
            
            match readline {
                Ok(line) => {
                    // Ajouter à l'historique
                    let _ = self.editor.add_history_entry(line.as_str());
                    self.history.add(&line);
                    
                    // Traiter la ligne
                    if let Err(e) = self.process_line(line.trim()) {
                        println!("{}", self.context.formatter().format_error(&format!("{}", e)));
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C
                    println!("Interruption (Ctrl-C)");
                    continue;
                },
                Err(ReadlineError::Eof) => {
                    // Ctrl-D
                    println!("Fin de l'entrée (Ctrl-D)");
                    break;
                },
                Err(err) => {
                    println!("{}", self.context.formatter().format_error(&format!("Erreur: {}", err)));
                    break;
                }
            }
        }
        
        // Sauvegarder l'historique
        self.history.save()?;
        
        Ok(())
    }
    
    /// Traite une ligne entrée par l'utilisateur
    fn process_line(&mut self, line: &str) -> Result<()> {
        // Ignorer les lignes vides
        if line.is_empty() {
            return Ok(());
        }
        
        // Traiter les commandes spéciales
        if line.starts_with('.') {
            return self.process_special_command(&line[1..]);
        }
        
        // Traiter les requêtes HyperionQL
        commands::query::execute(&mut self.context, line)
    }
    
    /// Traite les commandes spéciales (commençant par '.')
    fn process_special_command(&mut self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        
        match parts[0] {
            "help" => {
                self.print_help();
            },
            "exit" | "quit" => {
                println!("Au revoir !");
                // Sauvegarder l'historique avant de quitter
                self.history.save()?;
                std::process::exit(0);
            },
            "connect" => {
                if parts.len() < 2 {
                    println!("{}", self.context.formatter().format_error("Usage: .connect <path>"));
                    return Ok(());
                }
                
                if let Some(path_str) = std::path::PathBuf::from(parts[1]).to_str() {
                    commands::connect::execute(&mut self.context, path_str)?;
                } else {
                    println!("{}", self.context.formatter().format_error("Chemin invalide"));
                }
            },
            "list" => {
                let prefix = if parts.len() >= 2 { Some(parts[1]) } else { None };
                commands::list::execute(&mut self.context, prefix)?;
            },
            "format" => {
                if parts.len() < 2 {
                    println!("{}", self.context.formatter().format_error("Usage: .format <text|json|table>"));
                    return Ok(());
                }
                
                match parts[1] {
                    "text" => self.context.set_format(crate::formatters::OutputFormat::Text),
                    "json" => self.context.set_format(crate::formatters::OutputFormat::Json),
                    "table" => self.context.set_format(crate::formatters::OutputFormat::Table),
                    _ => {
                        println!("{}", self.context.formatter().format_error("Format inconnu"));
                        return Ok(());
                    }
                }
                
                println!("{}", self.context.formatter().format_success(&format!("Format défini à {}", parts[1])));
            },
            "history" => {
                // Nouvelle commande pour afficher l'historique
                let search_pattern = if parts.len() >= 2 { Some(parts[1]) } else { None };
                
                let commands = match search_pattern {
                    Some(pattern) => self.history.search(pattern),
                    None => self.history.get_all().to_vec(),
                };
                
                if commands.is_empty() {
                    println!("Aucune commande dans l'historique.");
                } else {
                    for (i, cmd) in commands.iter().enumerate() {
                        println!("{}: {}", i + 1, cmd);
                    }
                }
            },
            _ => {
                println!("{}", self.context.formatter().format_error(&format!("Commande inconnue: {}", cmd)));
            }
        }
        
        Ok(())
    }
    
    /// Affiche l'aide
    fn print_help(&self) {
        println!("Commandes disponibles:");
        println!("  .help                   Affiche cette aide");
        println!("  .exit, .quit            Quitte le CLI");
        println!("  .connect <path>         Se connecte à une base de données");
        println!("  .list [prefix]          Liste les chemins (avec préfixe optionnel)");
        println!("  .format <text|json|table> Définit le format de sortie");
        println!("  .history [pattern]      Affiche l'historique des commandes (filtré par motif optionnel)");
        println!();
        println!("Toute autre entrée est traitée comme une requête HyperionQL.");
    }
}