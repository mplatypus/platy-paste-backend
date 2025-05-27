CREATE TABLE IF NOT EXISTS pastes (
    -- The unique ID for the paste.
    "id" BIGINT NOT NULL PRIMARY KEY,
    -- When the paste was created.
    "creation" TIMESTAMPTZ NOT NULL,
    -- Whether the paste has been modified.
    "edited" TIMESTAMPTZ,
    -- The expiry of the paste.
    "expiry" TIMESTAMPTZ,
    -- The total amount of views of the paste.
    "views" BIGINT NOT NULL,
    -- The maximum amount of views allowed for the paste.
    "max_views" BIGINT
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