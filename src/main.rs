//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use rayon::iter::Either;
use rayon::prelude::*;

use anyhow::Result;
use log::error;
use walkdir::WalkDir;

mod checksum;
mod db;

use crate::checksum::Checksum;

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
                    let checksum = Checksum::compute(path)?;
                    let r = if db.exists_by_len_and_checksum(path, &metadata, checksum)? {
                        // Only the metadata changed, nothing to do
                        Ok(None)
                    } else {
                        // Everything changed
                        Ok(Some(path))
                    };

                    // Update either way, to at least avoid computing checksums in the future
                    db.upsert_entry(&path, &metadata, checksum)
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
