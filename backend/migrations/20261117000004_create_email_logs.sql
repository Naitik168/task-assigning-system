-- Development "outbox": every email the API would send is recorded here
-- instead of being delivered over SMTP.
CREATE TABLE email_logs (
    id         UUID PRIMARY KEY,
    to_email   TEXT        NOT NULL,
    subject    TEXT        NOT NULL,
    body       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_email_logs_to_email_created_at ON email_logs (to_email, created_at DESC);
