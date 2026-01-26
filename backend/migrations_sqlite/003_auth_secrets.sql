-- Auth secrets table for sensitive values (e.g. JWT signing key).
-- Not exposed via GraphQL; backend use only.

CREATE TABLE IF NOT EXISTS auth_secrets (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
