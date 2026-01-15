-- Add unique constraint on message ID (primary key already ensures this, but named for semantic error mapping)
-- The id column is already the primary key, so we add a named unique constraint for error mapping
ALTER TABLE messages ADD CONSTRAINT messages_id_unique UNIQUE (id);

-- Add unique constraint on (conversation_id, sequence_number) pairs
-- This ensures no two messages in the same conversation can have the same sequence number
ALTER TABLE messages ADD CONSTRAINT messages_conversation_sequence_unique UNIQUE (conversation_id, sequence_number);
