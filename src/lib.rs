use color_eyre::eyre::{eyre, Result};
use dialoguer::{theme::ColorfulTheme, Password};
use serde::{Deserialize, Serialize};
use std::io;
use std::{env, fs, path::PathBuf};

/// Cryptographic functions and data structures used to decrypt database with TOTP entries
///
/// The official Aegis documentation for vault decryption and contents can be found
/// [here](https://github.com/beemdevelopment/Aegis/blob/master/docs/vault.md#aegis-vault).
mod crypto;

/// Module for generating TOTP codes
///
/// The official Aegis documentation for code generation can be found
/// [here](https://github.com/beemdevelopment/Aegis/blob/master/docs/vault.md#entries).
pub mod totp;

/// Database containing TOTP entries
#[derive(Debug, Deserialize, Serialize)]
pub struct Database {
    /// Database version
    version: u32,
    /// List of TOTP entries
    pub entries: Vec<Entry>,
}

/// TOTP entry with information used to generate one time codes
#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub r#type: totp::EntryType,
    // pub uuid: String,
    pub name: String,
    pub issuer: String,
    // pub note: String,
    // pub favorite: bool,
    // pub icon: String,
    pub info: totp::EntryInfo,
}

/// Encrypted Aegis vault backup
#[derive(Debug, Deserialize, Serialize)]
pub struct Vault {
    /// Backup version
    version: u32,
    /// Information to decrypt master key
    header: crypto::Header,
    /// Base64 decoded AES265 encrypted JSON
    db: String,
}

impl Vault {
    pub fn is_encrypted(&self) -> bool {
        self.header.is_set()
    }
}

/// Parse vault from JSON. A list of entries are returned.
pub fn parse_aegis_vault(vault_backup_contents: &str) -> Result<Vec<Entry>> {
    let db: Database = match serde_json::from_str(vault_backup_contents) {
        Ok(vault) => extract_database(vault)?,
        Err(_) => return Err(eyre!("Failed to parse vault file")),
    };

    if db.version != 2 {
        return Err(eyre!(format!(
            "Unsupported database version: {}",
            db.version
        )));
    }

    Ok(db.entries)
}

fn extract_database(vault: Vault) -> Result<Database> {
    if vault.version != 1 {
        return Err(eyre!(format!(
            "Unsupported vault version: {}",
            vault.version
        )));
    }

    // Dump database to file
    // let db_dump = serde_json::to_string_pretty(&vault)?;
    // let db_dump_filepath = PathBuf::from("/tmp/aegis-pass-dump.json");
    // fs::write(db_dump_filepath, db_dump)?;

    if !vault.is_encrypted() {
        // Database in vault is in plaintext, just parse the JSON
        return match serde_json::from_str(&vault.db) {
            Ok(db) => Ok(db),
            Err(_) => Err(eyre!("Failed to parse JSON")),
        };
    } else {
        // Database in vault is encrypted
        let password = get_password()?;
        let db = crypto::decrypt(password.as_str(), vault)?;

        return Ok(db);
    }
}

/// Get password from user
fn get_password() -> io::Result<String> {
    // TODO: Refactor out password filepath
    let home = env::var("HOME").expect("Failed to expand $HOME");
    let password_filepath = PathBuf::from(home).join(".config/aegis-pass.txt");

    if fs::metadata(&password_filepath).is_ok() {
        println!("Found password file");
        let password = fs::read_to_string(&password_filepath)?;
        return Ok(password.trim().to_string());
    } else {
        return Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Insert Aegis Password")
            .interact();
    }
}
