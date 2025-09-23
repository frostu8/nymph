-- Create the card document database
CREATE TABLE Card (
    Id SERIAL PRIMARY KEY,
    -- card guild location
    GuildId BIGINT NOT NULL,
    -- full card name
    Name VARCHAR(255) NOT NULL,
    -- card content
    Content TEXT NOT NULL,
    -- timestamps
    InsertedAt TIMESTAMP NOT NULL,
    UpdatedAt TIMESTAMP NOT NULL,

    UNIQUE (GuildId, Name)
)
