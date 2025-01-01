-- Copyright © 2024 Clément Joly
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--

CREATE TABLE files (
    path TEXT PRIMARY KEY NOT NULL,
    modified_since_epoch_sec REAL NOT NULL, -- Float, number of seconds (and nanoseconds) since UNIX epoch
    size INT NOT NULL,
    checksum BLOB NOT NULL
) STRICT;

CREATE INDEX files_path_idx ON files(path);
