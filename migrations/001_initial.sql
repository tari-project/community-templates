-- Templates discovered from the indexer
CREATE TABLE IF NOT EXISTS templates (
    template_address TEXT PRIMARY KEY,
    template_name TEXT NOT NULL,
    author_public_key TEXT NOT NULL,
    binary_hash TEXT NOT NULL,
    at_epoch INTEGER NOT NULL,
    metadata_hash TEXT,
    definition TEXT,
    code_size INTEGER,
    is_blacklisted INTEGER NOT NULL DEFAULT 0,
    is_featured INTEGER NOT NULL DEFAULT 0,
    feature_order INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Verified off-chain metadata submitted by authors
CREATE TABLE IF NOT EXISTS template_metadata (
    template_address TEXT PRIMARY KEY REFERENCES templates(template_address),
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    category TEXT,
    repository TEXT,
    documentation TEXT,
    homepage TEXT,
    license TEXT,
    logo_url TEXT,
    extra TEXT NOT NULL DEFAULT '{}',
    schema_version INTEGER NOT NULL,
    cbor_bytes BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Search indexes
CREATE INDEX IF NOT EXISTS idx_metadata_category ON template_metadata (category);
CREATE INDEX IF NOT EXISTS idx_templates_featured ON templates (is_featured, feature_order) WHERE is_featured = 1;
CREATE INDEX IF NOT EXISTS idx_templates_at_epoch ON templates (at_epoch);

-- Admin users
CREATE TABLE IF NOT EXISTS admins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
