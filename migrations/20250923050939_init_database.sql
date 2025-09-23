-- Create the card document database
CREATE TABLE card (
    id SERIAL PRIMARY KEY,
    -- card guild location
    guild_id BIGINT NOT NULL,
    -- full card name
    name VARCHAR(255) NOT NULL,
    -- card content
    content TEXT NOT NULL,
    -- timestamps
    inserted_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,

    UNIQUE (guild_id, name)
)
