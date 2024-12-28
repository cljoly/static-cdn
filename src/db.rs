//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs::Metadata;
use std::path::Path;
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::new(vec![M::up(include_str!("db/1_up.sql"))]));

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open() -> Result<Self> {
        let mut conn = Connection::open("./static-cdn.sqlite")?;

        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"normal")?;
        conn.pragma_update(None, "foreign_keys", &"on")?;

        MIGRATIONS.to_latest(&mut conn)?;

        Ok(Db { conn })
    }

    pub fn exists_by_metadata(&self, path: &Path, metadata: &Metadata) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            r#"SELECT *
            FROM files
            WHERE path = ?1 AND datetime = ?2 AND size = ?3"#,
        )?;
        let modified_since_epoch = metadata.modified()?.duration_since(UNIX_EPOCH)?;
        let len = metadata.len();
        let mut rows = stmt.query(params![
            path.to_str(),
            modified_since_epoch.as_secs_f64(),
            len,
        ])?;
        Ok(rows.next()?.is_some())
    }

    pub fn exists_by_hash(&self, path: &Path, metadata: &Metadata, hash: u64) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            r#"SELECT *
            FROM files
            WHERE path = ?1 AND size = ?2 AND checksum = ?3"#,
        )?;
        let len = metadata.len();
        let mut rows = stmt.query(params![path.to_str(), len, hash.to_le_bytes(),])?;
        Ok(rows.next()?.is_some())
    }

    pub fn upsert_entry(&self, path: &Path, metadata: &Metadata, hash: u64) -> Result<()> {
        let mut stmt = self.conn.prepare_cached(
            r#"INSERT OR REPLACE INTO files (path, datetime, size, checksum)
            VALUES (?1, ?2, ?3, ?4)"#,
        )?;
        let modified_since_epoch = metadata.modified()?.duration_since(UNIX_EPOCH)?;
        let len = metadata.len();
        let n = stmt
            .execute(params![
                path.to_str(),
                // The loss of precision is quite small, worste case we will trigger a hash
                modified_since_epoch.as_secs_f64(),
                len,
                // Need to store as bytes, because a u64 can be bigger than a i64 and sqlite only
                // supports i64 (https://www.sqlite.org/datatype3.html)
                hash.to_le_bytes(),
            ])
            .expect(&format!(
                "should be able to insert {path:?}, {modified_since_epoch:?}, {len:?}, {hash}"
            ));
        debug_assert_eq!(1, n, "exactly one row should change");
        Ok(())
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
