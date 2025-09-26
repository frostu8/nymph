-- The card ownership table
CREATE TABLE ownership (
    owner_id BIGINT NOT NULL,
    -- The id of the card the user has
    card_id INT NOT NULL REFERENCES card(id),

    UNIQUE (owner_id, card_id)
);
