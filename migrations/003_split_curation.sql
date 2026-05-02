-- Split admin-curated flags (featured / blacklisted) out of `templates` into a
-- dedicated `template_curation` table.
--
-- Motivation: the indexer-owned `templates` table is wiped on admin reindex,
-- which previously also nuked curation. Keeping curation in its own table
-- (with no FK to `templates`) makes it survive any future wipe by construction.
--
-- The new table intentionally has NO foreign key into `templates`:
--   - SQLite FK enforcement is off by default in this app, so a FK would be
--     decorative rather than enforced.
--   - More importantly, a FK with `ON DELETE CASCADE` would re-introduce the
--     bug we're fixing, and a strict FK would block the wipe entirely.
--
-- Curation rows for templates the indexer no longer serves are harmless
-- orphans: they cost a row each and re-attach automatically if the template
-- ever reappears. A future GC pass can prune them if it ever matters.

CREATE TABLE IF NOT EXISTS template_curation (
    template_address TEXT PRIMARY KEY,
    is_featured INTEGER NOT NULL DEFAULT 0,
    feature_order INTEGER,
    is_blacklisted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_curation_featured
    ON template_curation (is_featured, feature_order)
    WHERE is_featured = 1;

CREATE INDEX IF NOT EXISTS idx_curation_blacklisted
    ON template_curation (is_blacklisted)
    WHERE is_blacklisted = 1;

-- Migrate existing curation data. Only carry over rows that were actually
-- curated; defaults are stored implicitly for everything else.
INSERT INTO template_curation
    (template_address, is_featured, feature_order, is_blacklisted, created_at, updated_at)
SELECT
    template_address,
    is_featured,
    feature_order,
    is_blacklisted,
    created_at,
    updated_at
FROM templates
WHERE is_featured = 1 OR is_blacklisted = 1;

-- Drop the old index that references the columns we're about to remove.
DROP INDEX IF EXISTS idx_templates_featured;

-- SQLite >= 3.35 supports ALTER TABLE DROP COLUMN.
ALTER TABLE templates DROP COLUMN is_blacklisted;
ALTER TABLE templates DROP COLUMN is_featured;
ALTER TABLE templates DROP COLUMN feature_order;
