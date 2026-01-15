-- Remove unique constraints
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_conversation_sequence_unique;
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_id_unique;
