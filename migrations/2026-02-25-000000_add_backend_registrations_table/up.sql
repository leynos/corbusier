-- Add agent backend registration table for roadmap item 1.3.1
-- Follows corbusier-design.md §2.2.3 and §6.2.3.

CREATE TABLE backend_registrations (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    capabilities JSONB NOT NULL,
    backend_info JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT backend_registrations_status_check CHECK (
        status IN ('active', 'inactive')
    )
);

CREATE UNIQUE INDEX idx_backend_registrations_name
    ON backend_registrations (name);
