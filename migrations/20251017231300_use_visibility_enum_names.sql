-- create column
ALTER TABLE card ADD COLUMN visibility_tmp VARCHAR(16) NOT NULL DEFAULT 'private';

-- convert existing cards to new visibility enum
UPDATE
    card
SET
    visibility_tmp = (
        CASE visibility
            WHEN 0 THEN 'private'
            WHEN 1 THEN 'hidden'
            WHEN 2 THEN 'public'
            ELSE 'private'
        END
    );

-- remove old visibility
ALTER TABLE card DROP COLUMN visibility;
ALTER TABLE card RENAME COLUMN visibility_tmp TO visibility;
