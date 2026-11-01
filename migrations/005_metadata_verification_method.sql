-- Track which verification path was used when storing off-chain metadata:
--   - 'on_chain_hash': CBOR payload hashed to the on-chain `metadata_hash`
--   - 'author_signature': CBOR payload verified by the template author's Schnorr signature
ALTER TABLE template_metadata
    ADD COLUMN verification_method TEXT NOT NULL DEFAULT 'on_chain_hash'
    CHECK (verification_method IN ('on_chain_hash', 'author_signature'));
