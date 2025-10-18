CREATE TABLE card (
    id INTEGER PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    category_name VARCHAR(255),
    previous_id INTEGER REFERENCES card(id),
    visibility INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    inserted_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,

    UNIQUE (guild_id, name)
);

-- create the users table
CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    display_name VARCHAR(255) NOT NULL,
    -- marker flag for automated users
    managed BOOLEAN NOT NULL DEFAULT FALSE,
    inserted_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE TABLE ownership (
    card_id INTEGER NOT NULL REFERENCES card(id),
    owner_id INTEGER NOT NULL REFERENCES user(id),
    owned BOOLEAN NOT NULL DEFAULT FALSE,

    UNIQUE (card_id, owner_id)
);

-- discord authentication
CREATE TABLE discord_auth (
    user_id INTEGER NOT NULL UNIQUE REFERENCES user(id),
    discord_id BIGINT NOT NULL UNIQUE,
    inserted_at TIMESTAMP NOT NULL
);

-- api-key based authentication
CREATE TABLE api_auth (
    user_id INTEGER NOT NULL REFERENCES user(id),
    hash CHAR(64) NOT NULL UNIQUE,
    inserted_at TIMESTAMP NOT NULL
);
