CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Templates discovered from the indexer
CREATE TABLE templates (
    template_address TEXT PRIMARY KEY,
    template_name TEXT NOT NULL,
    author_public_key TEXT NOT NULL,
    binary_hash TEXT NOT NULL,
    at_epoch BIGINT NOT NULL,
    metadata_hash TEXT,
    definition JSONB,
    code_size BIGINT,
    is_blacklisted BOOLEAN NOT NULL DEFAULT FALSE,
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    feature_order INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Verified off-chain metadata submitted by authors
CREATE TABLE template_metadata (
    template_address TEXT PRIMARY KEY REFERENCES templates(template_address),
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tags TEXT[] NOT NULL DEFAULT '{}',
    category TEXT,
    repository TEXT,
    documentation TEXT,
    homepage TEXT,
    license TEXT,
    logo_url TEXT,
    extra JSONB NOT NULL DEFAULT '{}',
    schema_version INT NOT NULL,
    cbor_bytes BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Search indexes
CREATE INDEX idx_templates_name_trgm ON templates USING gin (template_name gin_trgm_ops);
CREATE INDEX idx_metadata_tags ON template_metadata USING gin (tags);
CREATE INDEX idx_metadata_category ON template_metadata (category);
CREATE INDEX idx_metadata_description_trgm ON template_metadata USING gin (description gin_trgm_ops);
CREATE INDEX idx_templates_featured ON templates (is_featured, feature_order) WHERE is_featured = TRUE;
CREATE INDEX idx_templates_at_epoch ON templates (at_epoch);

-- Admin users
CREATE TABLE admins (
    id SERIAL PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
