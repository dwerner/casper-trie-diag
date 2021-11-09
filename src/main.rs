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
    #[structopt(short = "n", name = "server to ask for historical auction data")]
    address: String,

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
                    if let Err(_) = print_auction_details(None, cl_value) {
                        continue;
                    }

                    let auction_key_str = trie_key.to_formatted_string();
                    println!("found auction state, will traverse blocks and get historical data using this URef {}", auction_key_str);

                    let client = retrieve_state::Client::new();
                    let url = format!("{}/rpc", opts.address);

                    let highest_block = retrieve_state::get_block(&client, &url, None)
                        .await?
                        .block
                        .expect("no highest block");
                    println!(
                        "fetched highest block at height: {}",
                        highest_block.header.height
                    );

                    println!("bytes, eras, total_recipients, total_delegator_stakes");
                    let mut last_auction_size = 0;

                    // we just want a general sense of auction growth, so step by 100 blocks at a time
                    for height in (1..highest_block.header.height)
                        .step_by(100)
                        // but also include the highest block
                        .chain([highest_block.header.height])
                    {
                        let block = retrieve_state::get_block(
                            &client,
                            &url,
                            Some(GetBlockParams {
                                block_identifier: BlockIdentifier::Height(height as u64),
                            }),
                        )
                        .await?
                        .block
                        .unwrap();
                        let state_root_hash = block.header.state_root_hash;
                        let auction_state = retrieve_state::get_item(
                            &client,
                            &url,
                            GetItemParams {
                                state_root_hash,
                                key: auction_key_str.clone(),
                                path: Default::default(),
                            },
                        )
                        .await?
                        .stored_value;

                        if let casper_node::types::json_compatibility::StoredValue::CLValue(
                            cl_value,
                        ) = auction_state
                        {
                            let auction_size = cl_value.inner_bytes().len();
                            if auction_size != last_auction_size {
                                last_auction_size = auction_size;
                                if let Err(err) =
                                    print_auction_details(Some(height as usize), cl_value)
                                {
                                    println!(
                                        "error printing auction details at height {} state_root_hash {} {:?}",
                                        height, state_root_hash, err
                                    );
                                    continue;
                                }
                            }
                        }
                    }
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
