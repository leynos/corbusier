-- Add named unique constraint on message ID for semantic error mapping.
-- The primary key already enforces uniqueness, but PostgreSQL's primary key
-- constraint is named 'messages_pkey' by default. This additional named
-- constraint allows the application to inspect constraint_name() on unique
-- violations and map them to domain-specific error types (DuplicateMessage
-- vs DuplicateSequence) based on the constraint that was violated.
ALTER TABLE messages ADD CONSTRAINT messages_id_unique UNIQUE (id);
