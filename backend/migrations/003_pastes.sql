CREATE TABLE IF NOT EXISTS pastes (
    "id" BIGINT NOT NULL PRIMARY KEY,
    "owner_token" CHAR(25) NOT NULL,
    "document_ids" TEXT NOT NULL,
    FOREIGN KEY ("owner_token") REFERENCES users("token")
    ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS documents (
    "id" BIGINT NOT NULL PRIMARY KEY,
    "owner_token" CHAR(25) NOT NULL,
    "paste_id" BIGINT NOT NULL,
    "type" TEXT NOT NULL,
    FOREIGN KEY ("owner_token") REFERENCES users("token")
    ON DELETE CASCADE,
    FOREIGN KEY ("paste_id") REFERENCES pastes("id")
    ON DELETE CASCADE
)