PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    next_serial INTEGER NOT NULL CHECK (next_serial BETWEEN 0 AND 1000000)
);
INSERT OR IGNORE INTO app_state(id, next_serial) VALUES (1, 0);

CREATE TABLE IF NOT EXISTS category_mappings (
    tag_a TEXT PRIMARY KEY CHECK(length(tag_a) = 1),
    name TEXT NULL
);

CREATE TABLE IF NOT EXISTS mnemonic_mappings (
    tag_a TEXT NOT NULL REFERENCES category_mappings(tag_a) ON UPDATE CASCADE ON DELETE CASCADE,
    tag_b INTEGER NOT NULL CHECK(tag_b BETWEEN 0 AND 99),
    name TEXT NULL,
    PRIMARY KEY(tag_a, tag_b)
);

CREATE TABLE IF NOT EXISTS elements (
    serial INTEGER PRIMARY KEY CHECK(serial BETWEEN 0 AND 999999),
    kind TEXT NOT NULL CHECK(kind IN ('item', 'container')),
    tag_a TEXT NOT NULL CHECK(length(tag_a) = 1),
    tag_b INTEGER NOT NULL CHECK(tag_b BETWEEN 0 AND 99),
    tag_c INTEGER NOT NULL CHECK(tag_c BETWEEN 0 AND 99),
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    quantity REAL NOT NULL DEFAULT 1 CHECK(quantity >= 0),
    unit TEXT NOT NULL DEFAULT '',
    parent_serial INTEGER NULL REFERENCES elements(serial),
    image_mime TEXT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TEXT NULL,
    CHECK(parent_serial IS NULL OR parent_serial != serial)
);

CREATE INDEX IF NOT EXISTS idx_elements_parent ON elements(parent_serial);
CREATE INDEX IF NOT EXISTS idx_elements_name ON elements(name);
CREATE INDEX IF NOT EXISTS idx_elements_deleted ON elements(deleted_at);
