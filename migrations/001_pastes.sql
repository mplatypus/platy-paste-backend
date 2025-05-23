CREATE TABLE IF NOT EXISTS pastes (
    -- The unique ID for the paste.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- Whether the paste has been modified.
    "edited" BOOLEAN NOT NULL,
    -- The expiry of the paste.
    "expiry" TIMESTAMP WITH TIME ZONE
);

CREATE TABLE IF NOT EXISTS paste_tokens (
    -- The paste ID attached to the token.
    "paste_id" BIGINT NOT NULL PRIMARY KEY,
    -- The token for the paste.
    "token" TEXT NOT NULL UNIQUE,
    -- Foreign key that deletes the paste token when the paste ID (owner) gets deleted.
    FOREIGN KEY ("paste_id") REFERENCES pastes("id") ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS documents (
    -- The ID of the document.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- The paste that owns this document.
    "paste_id" BIGINT NOT NULL,
    -- The type of the documents contents.
    "type" TEXT NOT NULL,
    -- The name of the document.
    "name" TEXT NOT NULL,
    -- The size of the document.
    "size" BIGINT NOT NULL,
    -- Foreign key that deletes the paste token when the paste ID (owner) gets deleted.
    FOREIGN KEY ("paste_id") REFERENCES pastes("id") ON DELETE CASCADE
);