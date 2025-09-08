# Account Database Comparison Tool

A Rust tool to compare account data between Shardeum archiver and node databases.

## Overview

This tool compares account balance and nonce data between:
- **Archiver database**: Single `accounts.sqlite3` file with account data
- **Node databases**: Multiple `shardeum.sqlite` files in node instance folders

## Features

- ✅ Handles different account types (regular accounts vs special accounts like Foundation)
- ✅ Recursively discovers node databases in instance folders
- ✅ Compares balance and nonce values
- ✅ Reports mismatches with detailed information
- ✅ Provides summary statistics with match rates
- ✅ Optional verbose mode to show all comparisons

## Usage

Build using cargo:
```bash
cargo build
```

Run the tool:
```bash
# Basic usage (shows only mismatches)
./target/debug/shardeum-db-comparison -a <archiver_db_path> -n <nodes_folder_path>

# Verbose mode (shows all account comparisons)
./target/debug/shardeum-db-comparison -a <archiver_db_path> -n <nodes_folder_path> -v

# Save accounts spread in a CSV file
./target/debug/shardeum-db-comparison -a <archiver_db_path> -n <nodes_folder_path> -o <output_csv_path>
```

### Example

```bash
# Compare archiver DB with node instances
./target/debug/shardeum-db-comparison -a ../archiver-db-4000/accounts.sqlite3 -n ../shardeum/instances

# Show all comparisons (verbose)
./target/debug/shardeum-db-comparison -a ../archiver-db-4000/accounts.sqlite3 -n ../shardeum/instances -v
```

## Command Line Options

| Option | Description |
|--------|-------------|
| `-a, --archiver-db` | Path to archiver database file |
| `-n, --nodes-folder` | Path to folder containing node instances |
| `-v, --verbose` | Print all data (not just mismatches) |
| `-h, --help` | Show help message |

## Database Structure

### Archiver Database (`accounts.sqlite3`)
- **Table**: `accounts`
- **Columns**: `accountId`, `data` (JSON), `timestamp`, `hash`, `cycleNumber`, `isGlobal`

### Node Database (`shardeum.sqlite`)
- **Table**: `accountsEntry`
- **Columns**: `accountId`, `data` (JSON), `timestamp`

## JSON Data Format

The tool handles two types of account data:

### Regular Accounts
```json
{
  "account": {
    "balance": {"dataType": "bi", "value": "36437282f210de4b50"},
    "nonce": {"dataType": "bi", "value": "1"},
    "codeHash": {...},
    "storageRoot": {...}
  },
  "accountType": 0,
  "ethAddress": "0x...",
  "hash": "...",
  "timestamp": 1746544352826
}
```

### Special Accounts (e.g., Foundation)
```json
{
  "accountType": 13,
  "hash": "...",
  "id": "...",
  "name": "Foundation",
  "nonce": 7,
  "timestamp": 1746652080000
}
```

## Output

### Summary Example
```
=== SUMMARY ===
Total comparisons: 561392
Mismatches found: 0
Match rate: 100.00%
```

### Mismatch Example
```
Account ID: abc123...
Node: shardus-instance-9001
  Archiver - Balance: 1000000, Nonce: 5
  Node     - Balance: 1000000, Nonce: 6
  STATUS: MISMATCH
    - Nonce mismatch
```

## Building

```bash
cargo build
```

## Dependencies

- `rusqlite` - SQLite database access
- `serde`/`serde_json` - JSON parsing
- `clap` - Command line parsing
- `walkdir` - Recursive directory traversal
- `anyhow` - Error handling