-- Copyright © 2025 Clément Joly
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--

-- It turns out that SQLite automatically creates an index for this column, even
-- if one already exists, because `path` is a primary key. See
-- https://www.sqlite.org/fileformat2.html#intschema for details
DROP INDEX files_path_idx;
