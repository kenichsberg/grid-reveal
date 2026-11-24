-- What this installation calls itself.
--
-- The application name was a string in the client, which left a group running
-- this for themselves with no way to call it what they call it. It is the
-- first configuration the database owns: everything instance-wide until now
-- was an environment variable read once at boot, and an environment variable
-- cannot be changed from inside the application, which is the whole point.
--
-- One row, enforced rather than hoped for: a boolean primary key that may only
-- be TRUE admits exactly one. The row is seeded here, so every read finds one
-- and no handler carries a "what if it is unset" branch.
--
-- The shape constraint mirrors the API's validator, as 0009 does for the
-- per-screen labels. Blank is not a name.
CREATE TABLE site_settings (
    id         BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    app_name   TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT site_settings_app_name_shape
        CHECK (btrim(app_name) <> '' AND char_length(app_name) <= 60)
);

INSERT INTO site_settings (app_name) VALUES ('Grid Reveal');
