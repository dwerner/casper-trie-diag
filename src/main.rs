use std::path::{Path, PathBuf};

use structopt::StructOpt;

use retrieve_state::offline::load_execution_engine;
use casper_node::types::JsonBlock;
use casper_node::crypto::hash::Digest;
use casper_execution_engine::shared::newtypes::CorrelationId;
use casper_types::bytesrepr::ToBytes;
use reqwest::Client;


#[derive(Debug, StructOpt)]
struct Opts {

    #[structopt(short="p", default_value="data.lmdb")]
    lmdb_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    let mut client = Client::new();
    let opts = Opts::from_args();
    println!("Downloading highest block...");
    let highest_block: JsonBlock = retrieve_state::get_block(&mut client, None)
        .await?
        .block
        .unwrap();
    let state_root_hash = highest_block.header.state_root_hash;

    let (engine_state, _lmdb_environment) = load_execution_engine(&opts.lmdb_path, state_root_hash.into())?;

    let mut largest_trie = 0;
    while let Ok(Some(trie)) = engine_state.read_trie(CorrelationId::new(), state_root_hash.into())  {
        let serialized_len = trie.serialized_length();
        if largest_trie < serialized_len {
            println!("found new largest trie with len {}", serialized_len);
            largest_trie = serialized_len;
        }
    }

    Ok(())
}
