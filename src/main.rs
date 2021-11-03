use std::{collections::HashMap, path::PathBuf};

use casper_execution_engine::storage::trie::Trie;
use casper_hashing::Digest;
use casper_types::{bytesrepr::FromBytes, Key, StoredValue};
use lmdb::{Cursor, DatabaseFlags, Environment, EnvironmentFlags, Transaction};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(short = "p", name = "Path to LMDB data file.")]
    lmdb_path: PathBuf,

    #[structopt(
        short = "d",
        name = r#"Database name.

            For storage.lmdb, it should be one of:
                block_header
                block_metadata
                deploys
                deploy_metadata
                transfer
                state_store
                block_body

            For data.lmdb, it should be TRIE_STORE

    "#
    )]
    db_name: String,
}

fn main() -> Result<(), anyhow::Error> {
    let opts = Opts::from_args();
    let env = Environment::new()
        // Set the flag to manage our own directory like in the storage component.
        .set_flags(EnvironmentFlags::NO_SUB_DIR)
        .set_max_dbs(1)
        .open(&opts.lmdb_path)?;

    let db = env.create_db(Some(&opts.db_name), DatabaseFlags::empty())?;
    println!(
        "Scanning LMDB data file: {:?}\ndatabase name: {}",
        opts.lmdb_path, opts.db_name
    );

    let txn = env.begin_ro_txn()?;
    let mut cursor = txn.open_ro_cursor(db)?;
    let mut record_count = 0;
    let mut largest_record = 0;

    let mut trie = HashMap::new();
    for (key, value) in cursor.iter() {
        if opts.db_name == "TRIE_STORE" {
            let (key, _rest) = Digest::from_bytes(key)?;
            let (value, _remainder) = Trie::<Key, StoredValue>::from_bytes(value)?;
            trie.insert(key, value);
        }
        record_count += 1;
        let serialized_len = value.len();
        if largest_record < serialized_len {
            println!("found new largest DB entry with len {}", serialized_len);
            largest_record = serialized_len;
        }
    }
    let json = serde_json::to_string_pretty(&trie)?;
    println!("{}", json);
    println!("processed {} db records total", record_count);

    Ok(())
}
