use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "account_db_compare")]
#[command(about = "Compare accounts data between archiver and node databases")]
struct Args {
    #[arg(short = 'a', long, help = "Path to archiver database file")]
    archiver_db: PathBuf,
    
    #[arg(short = 'n', long, help = "Path to folder containing node instances")]
    nodes_folder: PathBuf,
    
    #[arg(short = 'v', long, help = "Print all data (not just mismatches)", default_value = "false")]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AccountData {
    Regular {
        account: Account,
        #[serde(rename = "accountType")]
        account_type: i32,
        #[serde(rename = "ethAddress")]
        eth_address: Option<String>,
        hash: String,
        timestamp: i64,
    },
    Special {
        #[serde(rename = "accountType")]
        account_type: i32,
        hash: String,
        id: String,
        name: Option<String>,
        nonce: Option<i64>,
        timestamp: i64,
        #[serde(flatten)]
        other_fields: serde_json::Map<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Account {
    balance: DataValue,
    #[serde(rename = "codeHash")]
    code_hash: DataValue,
    nonce: DataValue,
    #[serde(rename = "storageRoot")]
    storage_root: DataValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataValue {
    #[serde(rename = "dataType")]
    data_type: String,
    value: String,
}

#[derive(Debug)]
struct AccountEntry {
    account_id: String,
    data: AccountData,
    node_path: Option<String>,
}

impl AccountEntry {
    fn get_balance(&self) -> Option<&str> {
        match &self.data {
            AccountData::Regular { account, .. } => Some(&account.balance.value),
            AccountData::Special { .. } => None,
        }
    }
    
    fn get_nonce(&self) -> String {
        match &self.data {
            AccountData::Regular { account, .. } => account.nonce.value.clone(),
            AccountData::Special { nonce, .. } => nonce.map_or("N/A".to_string(), |n| n.to_string()),
        }
    }
    
    fn is_comparable(&self) -> bool {
        matches!(self.data, AccountData::Regular { .. })
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let archiver_accounts = load_archiver_accounts(&args.archiver_db)
        .context("Failed to load archiver accounts")?;
    
    let node_accounts = load_node_accounts(&args.nodes_folder)
        .context("Failed to load node accounts")?;
    
    compare_accounts(&archiver_accounts, &node_accounts, args.verbose);
    
    Ok(())
}

fn load_archiver_accounts(db_path: &Path) -> Result<HashMap<String, AccountEntry>> {
    let conn = Connection::open(db_path)
        .context("Failed to open archiver database")?;
    
    let mut stmt = conn.prepare("SELECT accountId, data FROM accounts")
        .context("Failed to prepare archiver query")?;
    
    let mut accounts = HashMap::new();
    
    let rows = stmt.query_map([], |row| {
        let account_id: String = row.get(0)?;
        let data_str: String = row.get(1)?;
        Ok((account_id, data_str))
    })?;
    
    for row in rows {
        let (account_id, data_str) = row?;
        match serde_json::from_str::<AccountData>(&data_str) {
            Ok(data) => {
                let entry = AccountEntry {
                    account_id: account_id.clone(),
                    data,
                    node_path: None,
                };
                if entry.is_comparable() {
                    accounts.insert(account_id, entry);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse archiver account data for {}: {}", account_id, e);
            }
        }
    }
    
    println!("Loaded {} accounts from archiver database", accounts.len());
    Ok(accounts)
}

fn load_node_accounts(nodes_folder: &Path) -> Result<HashMap<String, Vec<AccountEntry>>> {
    let mut all_accounts = HashMap::new();
    let mut node_count = 0;
    
    for entry in WalkDir::new(nodes_folder) {
        let entry = entry?;
        if entry.file_name() == "shardeum.sqlite" {
            let db_path = entry.path();
            let node_name = extract_node_name(&db_path);
            
            match load_single_node_accounts(&db_path, &node_name) {
                Ok(accounts) => {
                    node_count += 1;
                    println!("Loaded {} accounts from node: {}", accounts.len(), node_name);
                    
                    for account in accounts {
                        all_accounts
                            .entry(account.account_id.clone())
                            .or_insert_with(Vec::new)
                            .push(account);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load accounts from {}: {}", db_path.display(), e);
                }
            }
        }
    }
    
    println!("Loaded accounts from {} nodes", node_count);
    Ok(all_accounts)
}

fn load_single_node_accounts(db_path: &Path, node_name: &str) -> Result<Vec<AccountEntry>> {
    let conn = Connection::open(db_path)
        .context("Failed to open node database")?;
    
    let mut stmt = conn.prepare("SELECT accountId, data FROM accountsEntry")
        .context("Failed to prepare node query")?;
    
    let mut accounts = Vec::new();
    
    let rows = stmt.query_map([], |row| {
        let account_id: String = row.get(0)?;
        let data_str: String = row.get(1)?;
        Ok((account_id, data_str))
    })?;
    
    for row in rows {
        let (account_id, data_str) = row?;
        match serde_json::from_str::<AccountData>(&data_str) {
            Ok(data) => {
                let entry = AccountEntry {
                    account_id,
                    data,
                    node_path: Some(node_name.to_string()),
                };
                if entry.is_comparable() {
                    accounts.push(entry);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse node account data for {} in {}: {}", account_id, node_name, e);
            }
        }
    }
    
    Ok(accounts)
}

fn extract_node_name(db_path: &Path) -> String {
    db_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn compare_accounts(
    archiver_accounts: &HashMap<String, AccountEntry>,
    node_accounts: &HashMap<String, Vec<AccountEntry>>,
    verbose: bool,
) {
    println!("\n=== ACCOUNT COMPARISON ===\n");
    
    let mut mismatches = 0;
    let mut total_comparisons = 0;
    
    for (account_id, archiver_entry) in archiver_accounts {
        if let Some(node_entries) = node_accounts.get(account_id) {
            for node_entry in node_entries {
                total_comparisons += 1;
                
                let balance_match = archiver_entry.get_balance() == node_entry.get_balance();
                let nonce_match = archiver_entry.get_nonce() == node_entry.get_nonce();
                
                let has_mismatch = !balance_match || !nonce_match;
                
                if has_mismatch {
                    mismatches += 1;
                }
                
                if verbose || has_mismatch {
                    println!("Account ID: {}", account_id);
                    println!("Node: {}", node_entry.node_path.as_ref().unwrap_or(&"archiver".to_string()));
                    
                    let arch_balance = archiver_entry.get_balance().unwrap_or("N/A");
                    let arch_nonce = &archiver_entry.get_nonce();
                    let node_balance = node_entry.get_balance().unwrap_or("N/A");
                    let node_nonce = &node_entry.get_nonce();
                    
                    println!("  Archiver - Balance: {}, Nonce: {}", arch_balance, arch_nonce);
                    println!("  Node     - Balance: {}, Nonce: {}", node_balance, node_nonce);
                    
                    if has_mismatch {
                        println!("  STATUS: MISMATCH");
                        if !balance_match {
                            println!("    - Balance mismatch");
                        }
                        if !nonce_match {
                            println!("    - Nonce mismatch");
                        }
                    } else {
                        println!("  STATUS: MATCH");
                    }
                    println!();
                }
            }
        } else {
            if verbose {
                println!("Account ID: {} (ONLY IN ARCHIVER)", account_id);
                let arch_balance = archiver_entry.get_balance().unwrap_or("N/A");
                let arch_nonce = &archiver_entry.get_nonce();
                println!("  Balance: {}, Nonce: {}", arch_balance, arch_nonce);
                println!("  STATUS: NOT FOUND IN NODES\n");
            }
        }
    }
    
    for (account_id, node_entries) in node_accounts {
        if !archiver_accounts.contains_key(account_id) {
            if verbose {
                for node_entry in node_entries {
                    println!("Account ID: {} (ONLY IN NODE: {})", account_id, 
                            node_entry.node_path.as_ref().unwrap_or(&"unknown".to_string()));
                    let node_balance = node_entry.get_balance().unwrap_or("N/A");
                    let node_nonce = &node_entry.get_nonce();
                    println!("  Balance: {}, Nonce: {}", node_balance, node_nonce);
                    println!("  STATUS: NOT FOUND IN ARCHIVER\n");
                }
            }
        }
    }
    
    println!("=== SUMMARY ===");
    println!("Total comparisons: {}", total_comparisons);
    println!("Mismatches found: {}", mismatches);
    println!("Match rate: {:.2}%", 
            if total_comparisons > 0 { 
                (total_comparisons - mismatches) as f64 / total_comparisons as f64 * 100.0 
            } else { 
                0.0 
            });
}