-- Add per-function rustdoc to template_metadata.
--
-- `functions_json` is a JSON array of `{name, doc}` pairs, sourced from the new
-- `TemplateMetadata.functions` field added in tari_ootle_template_metadata 0.7.
-- Stored as a JSON string (mirroring `tags`) rather than a side table; queries
-- across function docs are not a use case today.
--
-- The 0.5 → 0.7 bump on the parent crate is *also* the bincode/ciborium →
-- minicbor cutover, so any previously-stored `cbor_bytes` is no longer
-- decodable. The denormalised columns are still readable, but the canonical
-- blob would be a permanent landmine. Wipe metadata so authors resubmit under
-- the new encoding. Curation lives in its own table (003_split_curation) and
-- is preserved by construction.
DELETE FROM template_metadata;

ALTER TABLE template_metadata ADD COLUMN functions_json TEXT NOT NULL DEFAULT '[]';
