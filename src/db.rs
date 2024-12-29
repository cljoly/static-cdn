/* Copyright © 2024 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fs::Metadata;
use std::path::Path;
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use rusqlite::{params, Connection, Transaction};
use rusqlite_migration::{Migrations, M};

use crate::Checksum;

static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::new(vec![M::up(include_str!("db/1_up.sql"))]));

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open() -> Result<Self> {
        let mut conn = Connection::open("./static-cdn.sqlite")?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL; \
             PRAGMA synchronous = NORMAL; \
             PRAGMA locking_mode = EXCLUSIVE; \
             PRAGMA temp_store = MEMORY;",
        )?;

        MIGRATIONS.to_latest(&mut conn)?;

        Ok(Db { conn })
    }

    pub fn transaction(&mut self) -> Result<Transaction> {
        Ok(self.conn.transaction()?)
    }

    pub fn exists_by_metadata(
        &self,
        path: &Path,
        metadata_values: &MetadataValues,
    ) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            r#"SELECT *
            FROM files
            WHERE path = ?1 AND modified_since_epoch_sec = ?2 AND size = ?3"#,
        )?;
        let MetadataValues {
            modified_since_epoch_sec,
            size,
        } = metadata_values;
        let mut rows = stmt.query(params![path.to_str(), modified_since_epoch_sec, size,])?;
        Ok(rows.next()?.is_some())
    }

    pub fn exists_by_len_and_checksum(
        &self,
        path: &Path,
        metadata_values: &MetadataValues,
        checksum: Checksum,
    ) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            r#"SELECT *
            FROM files
            WHERE path = ?1 AND size = ?2 AND checksum = ?3"#,
        )?;
        let mut rows = stmt.query(params![path.to_str(), metadata_values.size, checksum,])?;
        Ok(rows.next()?.is_some())
    }
}

pub fn upsert_entry(
    tx: &Transaction,
    path: &Path,
    metadata_values: &MetadataValues,
    checksum: Checksum,
) -> Result<()> {
    let mut stmt = tx.prepare_cached(
        r#"INSERT OR REPLACE INTO files (path, modified_since_epoch_sec, size, checksum)
            VALUES (?1, ?2, ?3, ?4)"#,
    )?;
    let MetadataValues {
        modified_since_epoch_sec,
        size,
    } = metadata_values;
    let n = stmt
        .execute(params![
            path.to_str(),
            modified_since_epoch_sec,
            size,
            checksum,
        ])
        .expect(&format!(
            "should be able to insert {path:?}, {metadata_values:?}, {checksum:?}"
        ));
    debug_assert_eq!(1, n, "exactly one row should change for {path:?}");
    Ok(())
}

pub fn update_metadata(
    tx: &Transaction,
    path: &Path,
    metadata_values: &MetadataValues,
) -> Result<()> {
    let mut stmt = tx.prepare_cached(
        r#"UPDATE OR FAIL files
           SET modified_since_epoch_sec = ?2, size = ?3
           WHERE path = ?1
          "#,
    )?;
    let MetadataValues {
        modified_since_epoch_sec,
        size,
    } = metadata_values;
    let n = stmt
        .execute(params![&path.to_str(), modified_since_epoch_sec, size,])
        .expect(&format!(
            "should be able to update {path:?}, {metadata_values:?}"
        ));
    debug_assert_eq!(1, n, "exactly one row should be updated for {path:?}");
    Ok(())
}

// Holds the values for the metadata columns in the table
#[derive(Debug)]
pub struct MetadataValues {
    modified_since_epoch_sec: f64,
    size: u64,
}

impl From<&Metadata> for MetadataValues {
    fn from(value: &Metadata) -> Self {
        let modified_since_epoch = value
            .modified()
            .expect(
                "this program requires the underlying filesystem to record modification date and time",
            )
            .duration_since(UNIX_EPOCH)
            .expect(
                "files can’t have been modified before the UNIX epoch.",
            );

        Self {
            // The loss of precision due to the float is deemed small enough (empirically, less
            // than 150 ns of precision are lost)
            modified_since_epoch_sec: modified_since_epoch.as_secs_f64(),
            size: value.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations() -> Result<()> {
        Ok(MIGRATIONS.validate()?)
    }
}
