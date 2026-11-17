-- Users and roles
CREATE TYPE user_role AS ENUM ('admin', 'staff');

CREATE TABLE users (
    id              UUID PRIMARY KEY,
    full_name       TEXT        NOT NULL CHECK (length(trim(full_name)) > 0),
    email           TEXT        NOT NULL UNIQUE CHECK (email = lower(email)),
    hashed_password TEXT        NOT NULL,
    role            user_role   NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
