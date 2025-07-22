-- Add up migration script here
CREATE TABLE IF NOT EXISTS "topups" (
    topup_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    topup_no TEXT NOT NULL,
    topup_amount INTEGER NOT NULL,
    topup_method TEXT NOT NULL,
    topup_time TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_topups_user_id
        FOREIGN KEY(user_id)
        REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);