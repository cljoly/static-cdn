CREATE TABLE files (
    path TEXT PRIMARY KEY,
    datetime REAL NOT NULL,
    size INT NOT NULL,
    checksum BLOB NOT NULL
) STRICT;

CREATE INDEX files_path_idx ON files(path);
