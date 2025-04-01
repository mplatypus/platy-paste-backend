CREATE TABLE IF NOT EXISTS users (
    -- The ID of the user.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- The name of the user.
    "name" TEXT NOT NULL UNIQUE,
    -- The email of the user.
    "email" TEXT NOT NULL UNIQUE,
    -- The permissions of the user.
    "permissions" BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_secrets (
    -- The ID of the user.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- The password the user is using.
    "password" TEXT NOT NULL,
    -- Removes all sessions when the user that owns them gets deleted.
    FOREIGN KEY ("id") REFERENCES users("id") ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_sessions (
    -- The token for the session.
    "token" CHAR(25) NOT NULL PRIMARY KEY,
    -- The user ID used for the session.
    "id" BIGINT NOT NULL,
    -- The expiry of this session.
    "expiry" TIMESTAMP WITH TIME ZONE NOT NULL,
    -- Removes all sessions when the user that owns them gets deleted.
    FOREIGN KEY ("id") REFERENCES users("id") ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bots (
    -- The ID of the bot.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- The name of the bot.
    "name" TEXT NOT NULL UNIQUE,
    -- The owner of the bot.
    "owner_id" BIGINT NOT NULL,
    -- The token of the bot.
    "token" CHAR(25) NOT NULL UNIQUE,
    -- The permissions of the bot.
    "permissions" BIGINT NOT NULL,
    -- Removes all bot tokens when the user that owns them gets deleted.
    FOREIGN KEY ("owner_id") REFERENCES users("id") ON DELETE CASCADE
);