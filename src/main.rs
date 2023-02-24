use std::{collections::HashMap, fs::File, io::BufWriter, io::Write, path::PathBuf, time::Instant};

use casper_execution_engine::{
    shared::newtypes::CorrelationId, storage::trie::Trie, storage::trie_store::DeleteResult,
};
use casper_hashing::Digest;
use casper_types::{bytesrepr::FromBytes, Key, StoredValue};
use lmdb::{Cursor, DatabaseFlags, Environment, EnvironmentFlags, Transaction};
use retrieve_state::storage;
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

    #[structopt(
        short = "s",
        name = "State root hex (optional). If passed it will gather stats only for the given state root."
    )]
    state_root_hex: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let start = Instant::now();
    let opts = Opts::from_args();
    let env = Environment::new()
        // Set the flag to manage our own directory like in the storage component.
        .set_flags(EnvironmentFlags::NO_SUB_DIR)
        .set_max_dbs(1)
        .open(&opts.lmdb_path)?;

    let db = env.create_db(Some(&opts.db_name), DatabaseFlags::empty())?;
    println!(
        "Scanning LMDB data file: {:?}\ndatabase name: {}, state root: {:?}",
        opts.lmdb_path, opts.db_name, opts.state_root_hex,
    );

    let txn = env.begin_ro_txn()?;
    let mut record_count = 0;
    let mut largest_record = 0;

    let mut key_tags = HashMap::<String, usize>::new();
    let mut stored_value_tags = HashMap::<String, usize>::new();
    let mut trie_lengths = HashMap::<(String, String), Vec<usize>>::new();

    if opts.db_name == "TRIE_STORE" {
        let state_root_hex = opts
            .state_root_hex
            .expect("TRIE_STORE requires a state root hash to be passed.");

        let state_root = Digest::from_hex(&state_root_hex).expect("error parsing state root hex");

        let filename = format!("trie_report-{}.csv", state_root_hex);
        println!("Will write trie report for state root to {}", filename);
        let mut report_writer = BufWriter::new(File::create(filename).unwrap());

        let mut unvisited_nodes = vec![state_root];
        let mut deleted_era_info = 0;
        let mut new_root_hash = state_root.clone();
        let (engine_state, _env) = storage::load_execution_engine(
            opts.lmdb_path,
            retrieve_state::DEFAULT_MAX_DB_SIZE,
            new_root_hash,
            true,
        )?;
        while let Some(digest) = unvisited_nodes.pop() {
            let bytes = txn
                .get(db, &digest)
                .expect("provided state root does not exist in database");

            let byte_len = bytes.len();
            if largest_record < byte_len {
                println!("Found new largest trie DB entry with len {}", byte_len);
                largest_record = byte_len;
            }
            let (trie_node, _remainder) = Trie::<Key, StoredValue>::from_bytes(bytes)
                .expect("unable to deserialize trie node");

            match trie_node {
                Trie::Leaf {
                    key: trie_key,
                    value: trie_value,
                } => {
                    log_trie_leaf_stats(
                        trie_key,
                        trie_value,
                        &mut key_tags,
                        &mut stored_value_tags,
                        &mut trie_lengths,
                        byte_len,
                    );
                    if let Key::EraInfo(_) = trie_key {
                        // line in the sand with era where we had erainfo.
                        // node -> get switch block and it's root -> get era-id
                        // for any newer -> hit stable key
                        // older -> use legacy

                        match engine_state.delete_key(
                            CorrelationId::new(),
                            new_root_hash,
                            &trie_key,
                        ) {
                            Ok(DeleteResult::Deleted(root)) => {
                                deleted_era_info += 1;
                                new_root_hash = root;
                            }
                            Ok(delete) => {
                                panic!("failed to delete key {:?} - {:?}", trie_key, delete)
                            }
                            err => {
                                panic!("failed to delete key {:?} - {:?}", trie_key, err)
                            }
                        }
                    }
                }
                Trie::Node { pointer_block } => unvisited_nodes.append(
                    &mut pointer_block
                        .as_indexed_pointers()
                        .map(|(_, ptr)| ptr.into_hash())
                        .collect::<Vec<Digest>>(),
                ),
                Trie::Extension { affix: _, pointer } => unvisited_nodes.push(pointer.into_hash()),
            }
        }
        record_count += 1;

        println!("deleted {deleted_era_info} era info entries.");

        writeln!(report_writer, "key_tag, count").unwrap();

        for (key_tag, count) in key_tags {
            writeln!(report_writer, "\"{}\", {}", key_tag, count).unwrap();
        }

        writeln!(report_writer, "stored_value_tag, count").unwrap();
        for (stored_value_tag, count) in stored_value_tags {
            writeln!(report_writer, "\"{}\", {}", stored_value_tag, count).unwrap();
        }

        writeln!(
            report_writer,
            "key_tag, stored_value_tag, average_len, max_len, total_len"
        )
        .unwrap();
        for ((key_tag, stored_value_tag), lengths) in trie_lengths {
            if lengths.is_empty() {
                continue;
            }
            let total_len = lengths.iter().sum::<usize>();
            let average_len: usize = total_len / lengths.len();
            let max_len: usize = *lengths.iter().max().unwrap();
            writeln!(
                report_writer,
                "\"{}\", \"{}\", {}, {}, {}",
                key_tag, stored_value_tag, average_len, max_len, total_len
            )
            .unwrap();
        }
    } else {
        let mut cursor = txn.open_ro_cursor(db)?;
        for (_key, value) in cursor.iter() {
            record_count += 1;
            let serialized_len = value.len();
            if largest_record < serialized_len {
                println!("found new largest DB entry with len {}", serialized_len);
                largest_record = serialized_len;
            }
        }
    }

    println!("processed {} db records total", record_count);

    Ok(())
}

fn log_trie_leaf_stats(
    trie_key: Key,
    trie_value: StoredValue,
    key_tags: &mut HashMap<String, usize>,
    stored_value_tags: &mut HashMap<String, usize>,
    trie_lengths: &mut HashMap<(String, String), Vec<usize>>,
    byte_len: usize,
) {
    let key_tag = trie_key.type_string();
    let stored_value_tag = trie_value.type_name();
    *key_tags.entry(key_tag.clone()).or_default() += 1;
    *stored_value_tags
        .entry(stored_value_tag.clone())
        .or_default() += 1;
    let trie_length_values = trie_lengths.entry((key_tag, stored_value_tag)).or_default();
    trie_length_values.push(byte_len);
}
