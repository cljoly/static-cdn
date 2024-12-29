//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::fs::File;
use std::hash::Hasher as _;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use rayon::iter::Either;
use rayon::prelude::*;

use anyhow::Result;
use log::error;
use twox_hash::XxHash64;
use walkdir::WalkDir;

mod db;

const SEED: u64 = 0x431C_71C5_AD99_39B4;
const CHUNK_SIZE: usize = 1 << 16;

fn main() -> Result<ExitCode> {
    // Initialize the db early
    let _ = db::Db::open().unwrap();

    let mut args = env::args();
    let _ = args.next().unwrap(); // Throw away the binary’s name
    let root_dir = args.next().unwrap();
    let all_files = WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                Some(entry)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let (potential_updates, errors): (Vec<_>, Vec<_>) = all_files
        .par_iter()
        .map_init(
            || db::Db::open().unwrap(),
            |db, entry| -> Result<Option<&Path>> {
                let path = entry.path();
                let metadata = path.metadata()?;

                if !db.exists_by_metadata(path, &metadata)? {
                    let mut f = File::open(path)?;
                    let mut b = [0u8; CHUNK_SIZE];
                    let mut hasher = XxHash64::with_seed(SEED);
                    loop {
                        let n = f.read(&mut b)?;
                        // This will hash trailing null bytes, but it's fine: if a file differs only by null
                        // bytes, for our purpose, we can deem it equal and we use the size for
                        // further comparison anyway.
                        hasher.write(&b);
                        if n == 0 {
                            break;
                        }
                    }
                    let hash = hasher.finish();

                    let r = if db.exists_by_hash(path, &metadata, hash)? {
                        // Only the metadata changed, nothing to do
                        Ok(None)
                    } else {
                        // Everything changed
                        Ok(Some(path))
                    };

                    // Update either way, to at least avoid computing hashes in the future
                    db.upsert_entry(&path, &metadata, hash)
                        .expect("entry should be added without issues");

                    r
                } else {
                    // No changes, nothing to do
                    Ok(None)
                }
            },
        )
        .partition_map(|r| match r {
            Ok(optional_path) => Either::Left(optional_path),
            Err(e) => Either::Right(e),
        });

    for e in &errors {
        error!("error encountered: {e}")
    }

    potential_updates
        .into_iter()
        .filter_map(|u| u)
        .for_each(|u| println!("update: {u:?}"));

    Ok(if errors.len() > 0 {
        2.into()
    } else {
        ExitCode::SUCCESS
    })
}
