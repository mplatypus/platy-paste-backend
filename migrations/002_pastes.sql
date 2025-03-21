CREATE TABLE IF NOT EXISTS pastes (
    "id" BIGINT NOT NULL PRIMARY KEY, -- The ID of the paste.
    "owner_id" BIGINT, -- The owner ID that owns this post.
    "owner_token" CHAR(25), -- The bot token that owns this paste.
    "document_ids" TEXT NOT NULL -- The documents under this paste.
);

CREATE TABLE IF NOT EXISTS documents (
    "id" BIGINT NOT NULL PRIMARY KEY, -- The ID of the document.
    "paste_id" BIGINT NOT NULL, -- The paste that owns this document.
    "type" TEXT NOT NULL -- The type of the documents contents.
);