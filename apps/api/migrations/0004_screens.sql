-- A screen is a picture covered by a cols x rows grid of tiles.
-- `permutation` is the fixed reveal order: a shuffle of every tile index in
-- [0, cols*rows). Which tiles are revealed is DERIVED from the sum of entry
-- minutes against this ordering; removals are never stored individually.
CREATE TABLE screens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT        NOT NULL UNIQUE,
    title        TEXT        NOT NULL,
    picture_id   UUID        NOT NULL REFERENCES pictures (id) ON DELETE RESTRICT,
    cols         INTEGER     NOT NULL,
    rows         INTEGER     NOT NULL,
    unit_minutes INTEGER     NOT NULL,
    permutation  INTEGER[]   NOT NULL,
    archived_at  TIMESTAMPTZ,
    created_by   UUID        NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT screens_title_not_blank CHECK (btrim(title) <> ''),
    CONSTRAINT screens_grid_positive CHECK (cols > 0 AND rows > 0),
    CONSTRAINT screens_unit_positive CHECK (unit_minutes > 0),
    CONSTRAINT screens_permutation_covers_grid
        CHECK (cardinality(permutation) = cols * rows)
);

CREATE INDEX screens_picture_id_idx ON screens (picture_id);
