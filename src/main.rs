use std::{collections::HashMap, path::PathBuf};

use casper_execution_engine::storage::trie::Trie;
use casper_hashing::Digest;
use casper_types::{
    bytesrepr::FromBytes, system::auction::SeigniorageRecipientsSnapshot, Key, StoredValue,
};
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

    for (key, value) in cursor.iter() {
        if opts.db_name == "TRIE_STORE" {
            let (key, _rest) = Digest::from_bytes(key)?;
            let (trie, _remainder) = Trie::<Key, StoredValue>::from_bytes(value)?;
            if let Trie::Leaf {
                key: trie_key,
                value: trie_value,
            } = trie
            {
                if let StoredValue::CLValue(cl_value) = trie_value {
                    let (_, bytes) = cl_value.destructure();
                    let bytes = bytes.inner_bytes();
                    let (snapshot, leftover) =
                        match SeigniorageRecipientsSnapshot::from_bytes(bytes) {
                            Ok(snapshot) => snapshot,
                            Err(_) => continue,
                        };
                    println!(
                        "found seignorage snapshot at key {} (value {} bytes)",
                        trie_key,
                        bytes.len(),
                    );
                    println!(
                        "{} eras in snapshot: {:?}",
                        snapshot.keys().len(),
                        snapshot.keys(),
                    );
                    println!(
                        "{} recipients in all snapshots",
                        snapshot
                            .values()
                            .map(|recipients| recipients.len())
                            .sum::<usize>()
                    );

                    println!(
                        "{} delegator stakes all snapshots",
                        snapshot
                            .values()
                            .map(|recipients| recipients
                                .values()
                                .map(|recipient| recipient.delegator_stake().len())
                                .sum::<usize>())
                            .sum::<usize>()
                    );

                    /*
                    for (era, recipients) in snapshot {
                        println!("era {} with {} recipients", era, recipients.len());
                        for (public_key, recipient) in recipients {
                            println!(
                                "public key {} recipient delegation rate:{: >8} stake:{: >32} delegators:{: >8}",
                                public_key,
                                recipient.delegation_rate(),
                                recipient.stake(),
                                recipient.delegator_stake().len(),
                            );
                        }
                    }
                    */
                }
            }
        }
        record_count += 1;
        let serialized_len = value.len();
        if largest_record < serialized_len {
            println!("found new largest DB entry with len {}", serialized_len);
            largest_record = serialized_len;
        }
    }
    println!("processed {} db records total", record_count);

    Ok(())
}
