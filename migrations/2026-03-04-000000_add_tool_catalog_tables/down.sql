-- Rollback tool catalog, audit trail, and log metadata tables (roadmap 2.1.2).
-- Drop tables first (cascades triggers), then the helper function.

DROP TABLE IF EXISTS tool_log_metadata;
DROP TABLE IF EXISTS tool_call_audit_log;
DROP TABLE IF EXISTS mcp_tool_catalog;
DROP FUNCTION IF EXISTS update_mcp_tool_catalog_updated_at();
