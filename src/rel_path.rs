/* Copyright © 2024 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use rusqlite::ToSql;

/// The database should hold paths relative to the folder walked through and the path part of urls
/// is also the relative path to that folder. Therefore, it makes sense to work most of the time
/// with those relative paths, as strings. The pair of types [`DbPath`] and [`DbPathBuilder`]
/// should make it easier to get that right, by keeping the root folder and returning relative
/// paths each time it's called. It also enforces that the right type is passed to DB functions.
#[derive(Debug)]
pub struct RelPath {
    // This is a relative path, to the same root folder, i.e. the base folder used for the walk
    rel_path: String,
}

impl RelPath {
    /// Returns a path relative to the root folder walked through
    pub fn get_relative_path(&self) -> &str {
        &self.rel_path
    }
}

impl ToSql for RelPath {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        ToSql::to_sql(&self.rel_path)
    }
}

pub struct RelPathBuilder<'a> {
    root_folder: &'a Path,
}

impl<'a> RelPathBuilder<'a> {
    pub fn new<P>(root_folder: &'a P) -> Self
    where
        P: AsRef<Path> + ?Sized,
    {
        Self {
            root_folder: root_folder.as_ref(),
        }
    }

    pub fn db_path<P>(&'a self, child: &'a P) -> RelPath
    where
        P: AsRef<Path> + ?Sized,
    {
        let rel_path = child.as_ref().strip_prefix(self.root_folder).expect("directories are walked in from the root folder, they should be relative. This is a bug, please report");
        debug_assert!(
            rel_path.is_relative(),
            "{rel_path:?} should be relative for storage in DB"
        );

        RelPath {
            rel_path: format!("{rel_path:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn db_path_not_relative_to_root() {
        // This should not happen because we walk from the root dir, but still, test that this
        // panics
        let _ = RelPathBuilder::new("/made_up/for_testing")
            .db_path("./some_other_folder/some_other_file");
    }
}
