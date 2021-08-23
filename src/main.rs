use std::path::PathBuf;

use lmdb::{Cursor, DatabaseFlags, Environment, EnvironmentFlags, Transaction};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(short = "p", name = "Path to data.lmdb")]
    lmdb_path: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let opts = Opts::from_args();
    let env = Environment::new()
        // Set the flag to manage our own directory like in the storage component.
        .set_flags(EnvironmentFlags::NO_SUB_DIR)
        .set_max_dbs(1)
        .open(&opts.lmdb_path)?;

    let db = env.create_db(Some("TRIE_STORE"), DatabaseFlags::empty())?;

    let txn = env.begin_ro_txn()?;
    let mut cursor = txn.open_ro_cursor(db)?;
    let mut largest_trie = 0;

    for (_key, value) in cursor.iter() {
        let serialized_len = value.len();
        if largest_trie < serialized_len {
            println!("found new largest trie with len {}", serialized_len);
            largest_trie = serialized_len;
        }
    }

    Ok(())
}
