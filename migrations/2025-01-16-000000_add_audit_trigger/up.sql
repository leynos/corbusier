-- Add audit trigger to capture session context on messages table operations
-- Follows corbusier-design.md audit trail requirements

-- Function to capture audit context from session variables
CREATE OR REPLACE FUNCTION capture_audit_context()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_logs (
        table_name,
        operation,
        row_id,
        old_values,
        new_values,
        correlation_id,
        causation_id,
        user_id,
        session_id,
        application_name
    ) VALUES (
        TG_TABLE_NAME,
        TG_OP,
        CASE WHEN TG_OP = 'DELETE' THEN OLD.id ELSE NEW.id END,
        CASE WHEN TG_OP IN ('UPDATE', 'DELETE') THEN to_jsonb(OLD) ELSE NULL END,
        CASE WHEN TG_OP IN ('INSERT', 'UPDATE') THEN to_jsonb(NEW) ELSE NULL END,
        NULLIF(current_setting('app.correlation_id', true), '')::uuid,
        NULLIF(current_setting('app.causation_id', true), '')::uuid,
        NULLIF(current_setting('app.user_id', true), '')::uuid,
        NULLIF(current_setting('app.session_id', true), '')::uuid,
        current_setting('application_name', true)
    );

    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

-- Trigger on messages table for all DML operations
CREATE TRIGGER messages_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON messages
    FOR EACH ROW
    EXECUTE FUNCTION capture_audit_context();
