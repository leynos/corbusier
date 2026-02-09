-- Revert agent sessions, handoffs, and context snapshots

-- Drop triggers first
DROP TRIGGER IF EXISTS agent_session_ended_at_trigger ON agent_sessions;
DROP FUNCTION IF EXISTS update_agent_session_ended_at();

-- Drop foreign key constraints that reference handoffs
ALTER TABLE agent_sessions DROP CONSTRAINT IF EXISTS agent_sessions_initiated_by_handoff_fk;
ALTER TABLE agent_sessions DROP CONSTRAINT IF EXISTS agent_sessions_terminated_by_handoff_fk;

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS context_snapshots;
DROP TABLE IF EXISTS handoffs;
DROP TABLE IF EXISTS agent_sessions;
