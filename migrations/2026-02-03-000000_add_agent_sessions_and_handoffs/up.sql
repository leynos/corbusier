-- Add agent sessions, handoffs, and context snapshots for context preservation
-- across agent transitions. Follows corbusier-design.md §4.2.1.1 and §2.2.1.

-- Agent sessions track contiguous periods where a single agent backend handles
-- turns within a conversation.
CREATE TABLE agent_sessions (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    agent_backend VARCHAR(100) NOT NULL,
    start_sequence BIGINT NOT NULL,
    end_sequence BIGINT,
    turn_ids JSONB NOT NULL DEFAULT '[]',
    initiated_by_handoff UUID,
    terminated_by_handoff UUID,
    context_snapshots JSONB NOT NULL DEFAULT '[]',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    state VARCHAR(20) NOT NULL DEFAULT 'active',
    CONSTRAINT agent_sessions_state_check CHECK (
        state IN ('active', 'paused', 'handed_off', 'completed', 'failed')
    )
);

-- Handoffs track transfers of conversation control between agent backends,
-- preserving audit trails and context references.
CREATE TABLE handoffs (
    id UUID PRIMARY KEY,
    source_session_id UUID NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    target_session_id UUID REFERENCES agent_sessions(id) ON DELETE SET NULL,
    prior_turn_id UUID NOT NULL,
    triggering_tool_calls JSONB NOT NULL DEFAULT '[]',
    source_agent VARCHAR(100) NOT NULL,
    target_agent VARCHAR(100) NOT NULL,
    reason TEXT,
    initiated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'initiated',
    CONSTRAINT handoffs_status_check CHECK (
        status IN ('initiated', 'accepted', 'completed', 'failed', 'cancelled')
    )
);

-- Context window snapshots capture the state visible to an agent at key moments,
-- enabling complete context reconstruction for auditing and handoff replay.
CREATE TABLE context_snapshots (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    sequence_start BIGINT NOT NULL,
    sequence_end BIGINT NOT NULL,
    message_summary JSONB NOT NULL,
    visible_tool_calls JSONB NOT NULL DEFAULT '[]',
    token_estimate BIGINT,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    snapshot_type VARCHAR(30) NOT NULL,
    CONSTRAINT context_snapshots_type_check CHECK (
        snapshot_type IN ('session_start', 'handoff_initiated', 'truncation', 'checkpoint')
    )
);

-- Add foreign key constraints for handoff references after both tables exist
ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_initiated_by_handoff_fk
    FOREIGN KEY (initiated_by_handoff) REFERENCES handoffs(id) ON DELETE SET NULL;

ALTER TABLE agent_sessions
    ADD CONSTRAINT agent_sessions_terminated_by_handoff_fk
    FOREIGN KEY (terminated_by_handoff) REFERENCES handoffs(id) ON DELETE SET NULL;

-- Indexes for common query patterns

-- Find all sessions for a conversation
CREATE INDEX idx_agent_sessions_conversation_id ON agent_sessions(conversation_id);

-- Find active session for a conversation
CREATE INDEX idx_agent_sessions_conversation_state ON agent_sessions(conversation_id, state)
    WHERE state = 'active';

-- Find sessions by agent backend
CREATE INDEX idx_agent_sessions_agent_backend ON agent_sessions(agent_backend);

-- Find handoffs by source session
CREATE INDEX idx_handoffs_source_session_id ON handoffs(source_session_id);

-- Find handoffs by conversation
CREATE INDEX idx_handoffs_conversation_id ON handoffs(conversation_id);

-- Find handoffs by target session
CREATE INDEX idx_handoffs_target_session_id ON handoffs(target_session_id)
    WHERE target_session_id IS NOT NULL;

-- Find handoffs by status (for monitoring incomplete handoffs)
CREATE INDEX idx_handoffs_status ON handoffs(status)
    WHERE status NOT IN ('completed', 'failed', 'cancelled');

-- Find snapshots by session
CREATE INDEX idx_context_snapshots_session_id ON context_snapshots(session_id);

-- Find snapshots by conversation and type
CREATE INDEX idx_context_snapshots_conversation_type ON context_snapshots(conversation_id, snapshot_type);

-- Trigger to auto-update ended_at when session state becomes terminal
CREATE OR REPLACE FUNCTION update_agent_session_ended_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.state IN ('handed_off', 'completed', 'failed')
        AND OLD.state IN ('active', 'paused') THEN
        NEW.ended_at := NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agent_session_ended_at_trigger
    BEFORE UPDATE ON agent_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_session_ended_at();
