-- Remove unique constraints added by this migration.
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_id_unique;
