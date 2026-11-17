-- Email two-factor login challenges.
-- Only an HMAC-SHA256 of the one-time code is stored, never the code itself.
CREATE TABLE login_challenges (
    id              UUID PRIMARY KEY,
    user_id         UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code_hash       TEXT        NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    consumed_at     TIMESTAMPTZ,
    failed_attempts INTEGER     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_login_challenges_user_id ON login_challenges (user_id);
