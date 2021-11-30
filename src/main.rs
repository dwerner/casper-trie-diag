use std::{collections::HashMap, net::IpAddr, ops::ControlFlow, path::PathBuf};

use casper_execution_engine::storage::trie::Trie;
use casper_hashing::Digest;
use casper_node::rpcs::{
    chain::{BlockIdentifier, GetBlockParams},
    state::GetItemParams,
};
use casper_types::{
    bytesrepr::FromBytes,
    system::auction::{self, SeigniorageRecipientsSnapshot},
    Key, StoredValue,
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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

    let mut key_tags = HashMap::<String, usize>::new();
    let mut stored_value_tags = HashMap::<String, usize>::new();
    let mut trie_lengths = HashMap::<(String, String), Vec<usize>>::new();

    for (key, value) in cursor.iter() {
        if opts.db_name == "TRIE_STORE" {
            let (_key, _rest) = Digest::from_bytes(key)?;
            let byte_len = value.len();
            let (trie, _remainder) = Trie::<Key, StoredValue>::from_bytes(value)?;
            if let Trie::Leaf {
                key: trie_key,
                value: trie_value,
            } = trie
            {
                let key_tag = trie_key.type_string();
                let stored_value_tag = trie_value.type_name();

                *key_tags.entry(key_tag.clone()).or_default() += 1;
                *stored_value_tags
                    .entry(stored_value_tag.clone())
                    .or_default() += 1;
                let trie_length_values =
                    trie_lengths.entry((key_tag, stored_value_tag)).or_default();
                trie_length_values.push(byte_len);
            }
        }
        record_count += 1;
        let serialized_len = value.len();
        if largest_record < serialized_len {
            println!("found new largest DB entry with len {}", serialized_len);
            largest_record = serialized_len;
        }
    }

    println!("key_tag, count");
    for (key_tag, count) in key_tags {
        println!("\"{}\", {}", key_tag, count);
    }

    println!("stored_value_tag, count");
    for (stored_value_tag, count) in stored_value_tags {
        println!("\"{}\", {}", stored_value_tag, count);
    }

    println!("key_tag, stored_value_tag, average_len, max_len");
    for ((key_tag, stored_value_tag), lengths) in trie_lengths {
        if lengths.is_empty() {
            continue;
        }
        let average_len: usize = lengths.iter().sum::<usize>() / lengths.len();
        let max_len: usize = *lengths.iter().max().unwrap();
        println!(
            "\"{}\", \"{}\", {}, {}",
            key_tag, stored_value_tag, average_len, max_len
        );
    }

    println!("processed {} db records total", record_count);

    Ok(())
}

fn print_auction_details(
    height: Option<usize>,
    cl_value: casper_types::CLValue,
) -> Result<(), anyhow::Error> {
    let bytes = cl_value.inner_bytes();
    let (snapshot, leftover) = SeigniorageRecipientsSnapshot::from_bytes(bytes)?;
    let eras = snapshot.keys();
    let total_recipients = snapshot
        .values()
        .map(|recipients| recipients.len())
        .sum::<usize>();
    let total_delegator_stakes = snapshot
        .values()
        .map(|recipients| {
            recipients
                .values()
                .map(|recipient| recipient.delegator_stake().len())
                .sum::<usize>()
        })
        .sum::<usize>();

    let height = match height {
        Some(height) => format!("block({})", height),
        None => "unknown".to_string(),
    };
    println!(
        "{}, {}, {:?}, {}, {}",
        height,
        bytes.len(),
        eras,
        total_recipients,
        total_delegator_stakes
    );
    Ok(())
}
