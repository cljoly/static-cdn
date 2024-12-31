/* Copyright © 2024 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fs::File;
use std::hash::Hasher as _;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use rusqlite::ToSql;
use twox_hash::XxHash64;

const SEED: u64 = 0x431C_71C5_AD99_39B4;
const CHUNK_SIZE: usize = 1 << 16;

#[derive(Debug, Default, Clone, Copy)]
pub struct Checksum {
    sum: [u8; 8],
}

impl From<u64> for Checksum {
    fn from(value: u64) -> Self {
        Self {
            sum: value.to_le_bytes(),
        }
    }
}

impl ToSql for Checksum {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        // Need to store as bytes, because a u64 can be bigger than a i64 and sqlite only
        // supports i64 (https://www.sqlite.org/datatype3.html)
        self.sum.to_sql()
    }
}

impl Checksum {
    pub fn compute(path: &Path) -> Result<Checksum> {
        let mut f = File::open(path)?;
        let mut b = [0u8; CHUNK_SIZE];
        let mut hasher = XxHash64::with_seed(SEED);
        loop {
            let n = f.read(&mut b)?;
            // This will hash trailing null bytes, but it's fine: if a file differs only by
            // null bytes, for our purpose, we can deem it equal and we use the size for
            // further comparison anyway.
            hasher.write(&b);
            if n == 0 {
                break;
            }
        }
        Ok(hasher.finish().into())
    }
}
