-- Remove audit trigger and function
DROP TRIGGER IF EXISTS messages_audit_trigger ON messages;
DROP FUNCTION IF EXISTS capture_audit_context();
