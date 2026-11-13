-- Accounts. Created only by administrators; there is no self-service signup.
CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT        NOT NULL,
    email         TEXT,
    password_hash TEXT        NOT NULL,
    is_admin      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_username_len CHECK (char_length(username) BETWEEN 3 AND 32)
);

-- Usernames are unique case-insensitively, so `Alice` and `alice` collide.
CREATE UNIQUE INDEX users_username_lower_key ON users (lower(username));
