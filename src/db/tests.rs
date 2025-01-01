/* Copyright © 2024-2025 Clément Joly
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;

use crate::rel_path::RelPathBuilder;

use anyhow::Result;

fn test_db_path() -> RelPath {
    RelPathBuilder::new("/made_up/for_testing")
        .db_path("/made_up/for_testing/some_other_folder/some_other_file")
}

#[test]
fn migrations() -> Result<()> {
    Ok(MIGRATIONS.validate()?)
}

#[test]
#[should_panic]
fn update_fails_when_nothing_exists() {
    let _ = open_transient().and_then(|mut c| {
        let _ = c.transaction().and_then(|tx| {
            // This should panic and nothing else can in this test
            let _ = update_metadata(&tx, &test_db_path(), &MetadataValues::default());
            Ok(())
        });
        Ok(())
    });
}

#[test]
fn insertion_and_checks() -> Result<()> {
    let db_path = test_db_path();
    let initial_metadata = MetadataValues {
        modified_since_epoch_sec: 12.,
        size: 10,
    };
    let updated_metadata = MetadataValues {
        size: 99,
        ..initial_metadata
    };
    let initial_checksum = Checksum::from(10);
    let updated_checksum = Checksum::from(20);
    let mut conn = open_transient()?;

    assert!(
        !exists_by_metadata(&mut conn, &db_path, &initial_metadata)?,
        "nothing should be inserted yet"
    );
    insta::assert_snapshot!("empty_table", read_all_files_rows(&conn));

    {
        let tx = conn.transaction()?;
        upsert_entry(&tx, &db_path, &initial_metadata, initial_checksum)?;
        tx.commit()?;
    }
    insta::assert_snapshot!("first_instert", read_all_files_rows(&conn));
    assert!(
        exists_by_metadata(&mut conn, &db_path, &initial_metadata)?,
        "should be inserted now"
    );
    assert!(
        exists_by_len_and_checksum(&mut conn, &db_path, &initial_metadata, initial_checksum)?,
        "should be inserted now, with the right checksum"
    );

    // Update
    {
        let tx = conn.transaction()?;
        upsert_entry(&tx, &db_path, &updated_metadata, updated_checksum)?;
        tx.commit()?;
    }
    insta::assert_snapshot!("after_update", read_all_files_rows(&conn));
    assert!(
        !exists_by_metadata(&mut conn, &db_path, &initial_metadata)?,
        "should not find the old version"
    );
    assert!(
        exists_by_metadata(&mut conn, &db_path, &updated_metadata)?,
        "should be updated"
    );
    assert!(
        exists_by_len_and_checksum(&mut conn, &db_path, &updated_metadata, updated_checksum)?,
        "should be updated, with the right checksum"
    );

    Ok(())
}
