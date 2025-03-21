CREATE TABLE IF NOT EXISTS users (
    "name" TEXT NOT NULL UNIQUE, -- The name of the user.
    "user_id" BIGINT NOT NULL PRIMARY KEY, -- The ID of the user.
    "permissions" BIGINT NOT NULL -- The permissions of the user.
);

CREATE TABLE IF NOT EXISTS bots (
    "name" TEXT NOT NULL UNIQUE, -- The name of the bot.
    "token" CHAR(25) NOT NULL PRIMARY KEY, -- The token of the bot.
    "permissions" BIGINT NOT NULL -- The permissions of the bot.
);