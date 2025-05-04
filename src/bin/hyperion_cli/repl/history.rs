use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use anyhow::Result;

/// Structure pour gérer l'historique des commandes
pub struct History {
    /// Chemin du fichier d'historique
    file_path: String,
    
    /// Commandes en mémoire
    commands: Vec<String>,
    
    /// Taille maximale de l'historique
    max_size: usize,
}

impl History {
    /// Crée un nouvel historique
    pub fn new(file_path: &str, max_size: usize) -> Self {
        History {
            file_path: file_path.to_string(),
            commands: Vec::new(),
            max_size,
        }
    }
    
    /// Charge l'historique depuis un fichier
    pub fn load(&mut self) -> Result<()> {
        // Vérifier si le fichier existe
        if !Path::new(&self.file_path).exists() {
            return Ok(());
        }
        
        // Ouvrir le fichier
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        
        // Lire les commandes
        self.commands.clear();
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                self.commands.push(line);
            }
        }
        
        // Limiter la taille
        if self.commands.len() > self.max_size {
            self.commands = self.commands[self.commands.len() - self.max_size..].to_vec();
        }
        
        Ok(())
    }
    
    /// Sauvegarde l'historique dans un fichier
    pub fn save(&self) -> Result<()> {
        // Créer ou ouvrir le fichier
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)?;
        
        // Écrire les commandes
        for cmd in &self.commands {
            writeln!(file, "{}", cmd)?;
        }
        
        Ok(())
    }
    
    /// Ajoute une commande à l'historique
    pub fn add(&mut self, command: &str) {
        // Ignorer les commandes vides
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        
        // Éviter les doublons consécutifs
        if let Some(last) = self.commands.last() {
            if last == command {
                return;
            }
        }
        
        // Ajouter la commande
        self.commands.push(command.to_string());
        
        // Limiter la taille
        if self.commands.len() > self.max_size {
            self.commands.remove(0);
        }
    }
    
    /// Récupère toutes les commandes
    pub fn get_all(&self) -> &[String] {
        &self.commands
    }
    
    /// Recherche des commandes correspondant à un motif
    pub fn search(&self, pattern: &str) -> Vec<String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.contains(pattern))
            .cloned()
            .collect()
    }
}