CREATE TABLE files (
    path TEXT PRIMARY KEY NOT NULL,
    modified_since_epoch_sec REAL NOT NULL, -- Float, number of seconds (and nanoseconds) since UNIX epoch
    size INT NOT NULL,
    checksum BLOB NOT NULL
) STRICT;

CREATE INDEX files_path_idx ON files(path);
