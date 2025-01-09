/* Copyright © 2024-2025 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use indicatif::ParallelProgressIterator;
use log::error;
use rayon::iter::Either;
use rayon::prelude::*;
use walkdir::WalkDir;

mod cdn;
mod checksum;
mod config;
mod db;
mod rel_path;
#[cfg(test)]
mod tests;

use crate::checksum::Checksum;

use self::db::MetadataValues;
use self::rel_path::{RelPath, RelPathBuilder};

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

    let config = config::load();
    println!("Scanning {}...", args.root_dir);
    let all_files = WalkDir::new(&args.root_dir)
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

    let mut conn = db::open()?;
    let db_path_builder = RelPathBuilder::new(&args.root_dir);

    println!("Detecting changes");
    // A Vec<()> takes no memory per element, but it's useful to count how many such elements there
    // are
    let ((unchanged, updates), (store, errors)): ((Vec<()>, Vec<_>), (Vec<_>, Vec<_>)) = all_files
        .par_iter()
        .progress()
        .map_init(
            || db::open().unwrap(),
            |conn, entry| -> Result<PathOutcome> {
                let path = entry.path();
                let db_path = db_path_builder.db_path(path);
                let metadata_values = MetadataValues::from(&path.metadata()?);

                if args.force_deep_check
                    || !db::exists_by_metadata(conn, &db_path, &metadata_values)?
                {
                    let checksum = Checksum::compute(path)?;
                    if db::exists_by_len_and_checksum(conn, &db_path, &metadata_values, checksum)? {
                        Ok(PathOutcome::UpdateMetdata(db_path, metadata_values))
                    } else {
                        Ok(PathOutcome::StoreAndInvalidate(
                            db_path,
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

    println!("Updating the cache");
    // Write operations are single-threaded in SQLite
    let tx = conn.transaction()?;
    for (path, metadata_values) in &updates {
        db::update_metadata(&tx, path, &metadata_values)?;
    }
    for (path, metadata_values, checksum) in &store {
        // TODO Coordinate this with calls to the CDN API
        db::upsert_entry(&tx, path, &metadata_values, *checksum)?;
    }
    tx.commit()?;

    for e in &errors {
        error!("error encountered: {e}")
    }

    dbg!(store.chunks(30).len());

    dbg!(store.chunks(30).count());
    // TODO Actually perform the update
    //.for_each(|u| println!("update: {u:?}"));

    log::debug!(
        "Summary: {} unchanged, {} with different metadata and {} changed files.",
        unchanged.len(),
        updates.len(),
        store.len()
    );
    println!("Total: {file_count} files.");
    Ok(if errors.len() > 0 {
        2.into()
    } else {
        ExitCode::SUCCESS
    })
}

// Control what do with the paths
enum PathOutcome {
    // Path is unchanged, nothing to do (no CDN or DB update)
    Skip,
    // Path medata have changed, but the checksum is the same, only update the DB
    UpdateMetdata(RelPath, MetadataValues),
    // Path checksum and metadata have changed, update both the DB and the CDN
    StoreAndInvalidate(RelPath, MetadataValues, Checksum),
}
