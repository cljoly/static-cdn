/* Copyright © 2024 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use rayon::iter::Either;
use rayon::prelude::*;

use anyhow::Result;
use log::error;
use walkdir::WalkDir;

mod checksum;
mod db;
#[cfg(test)]
mod tests;

use crate::checksum::Checksum;

use self::db::MetadataValues;

/// A CDN cache invalidation tool for your static site
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Directory holding the static site cached by the CDN
    #[arg()]
    root_dir: String,

    /// Whether to use fast change detection (relies on the filesystem metadata to detect some of the
    /// changes)
    #[arg(short, long, default_value_t = false)]
    force_deep_check: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    println!("{args:?}");
    let all_files = WalkDir::new(args.root_dir)
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
    let file_count = all_files.len();

    // A Vec<()> takes no memory per element, but it's useful to count how many such elements there
    // are
    let ((unchanged, updates), (store, errors)): ((Vec<()>, Vec<_>), (Vec<_>, Vec<_>)) = all_files
        .par_iter()
        .map_init(
            || db::Db::open().unwrap(),
            |db, entry| -> Result<PathOutcome> {
                let path = entry.path();
                let metadata_values = MetadataValues::from(&path.metadata()?);

                if args.force_deep_check || !db.exists_by_metadata(path, &metadata_values)? {
                    let checksum = Checksum::compute(path)?;
                    if db.exists_by_len_and_checksum(path, &metadata_values, checksum)? {
                        Ok(PathOutcome::UpdateMetdata(&path, metadata_values))
                    } else {
                        Ok(PathOutcome::StoreAndInvalidate(
                            &path,
                            metadata_values,
                            checksum,
                        ))
                    }
                } else {
                    Ok(PathOutcome::Skip)
                }
            },
        )
        .partition_map(|r| match r {
            Ok(PathOutcome::Skip) => Either::Left(Either::Left(())),
            Ok(PathOutcome::UpdateMetdata(p, mv)) => Either::Left(Either::Right((p, mv))),
            Ok(PathOutcome::StoreAndInvalidate(p, mv, c)) => {
                Either::Right(Either::Left((p, mv, c)))
            }
            Err(e) => Either::Right(Either::Right(e)),
        });

    println!("Storing...");
    // Insertion is single threaded in SQLite
    // TODO Coordinate this with calls to the CDN API
    let mut db = db::Db::open()?;
    let tx = db.transaction()?;
    for (path, metadata_values) in &updates {
        db::update_metadata(&tx, path, &metadata_values)?;
    }
    for (path, metadata_values, checksum) in &store {
        db::upsert_entry(&tx, path, &metadata_values, *checksum)?;
    }
    tx.commit()?;

    for e in &errors {
        error!("error encountered: {e}")
    }

    // Update either way, to at least avoid computing checksums in the future
    //db.upsert_entry(&path, &metadata, checksum)
    //    .expect("entry should be added without issues");

    store
        .iter()
        // TODO Actually perform the update
        .for_each(|u| println!("update: {u:?}"));

    println!(
        "Summary: {} unchanged, {} with different metadata and {} changed files.",
        unchanged.len(),
        updates.len(),
        store.len()
    );
    println!("Total:  {file_count} files.");
    Ok(if errors.len() > 0 {
        2.into()
    } else {
        ExitCode::SUCCESS
    })
}

// Control what do with the paths
enum PathOutcome<'p> {
    // Path is unchanged, nothing to do (no CDN or DB update)
    Skip,
    // Path medata have changed, but the checksum is the same, only update the DB
    UpdateMetdata(&'p Path, MetadataValues),
    // Path checksum and metadata have changed, update both the DB and the CDN
    StoreAndInvalidate(&'p Path, MetadataValues, Checksum),
}
