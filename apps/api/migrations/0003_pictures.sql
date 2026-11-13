-- Shared background picture library, visible to every administrator.
-- Soft delete hides a picture from the picker; existing screens keep using it.
CREATE TABLE pictures (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    storage_key TEXT        NOT NULL UNIQUE,
    width       INTEGER     NOT NULL,
    height      INTEGER     NOT NULL,
    uploaded_by UUID        NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ,
    CONSTRAINT pictures_dimensions_positive CHECK (width > 0 AND height > 0)
);

CREATE INDEX pictures_active_idx ON pictures (created_at DESC) WHERE deleted_at IS NULL;
