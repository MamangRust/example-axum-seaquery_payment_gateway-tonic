-- Add up migration script here
CREATE TABLE IF NOT EXISTS "transfers" (
    transfer_id SERIAL PRIMARY KEY,
    transfer_from INTEGER NOT NULL,
    transfer_to INTEGER NOT NULL,
    transfer_amount INTEGER NOT NULL DEFAULT 0,
    transfer_time TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_transfers_from
        FOREIGN KEY(transfer_from)
        REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT fk_transfers_to
        FOREIGN KEY(transfer_to)
        REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);