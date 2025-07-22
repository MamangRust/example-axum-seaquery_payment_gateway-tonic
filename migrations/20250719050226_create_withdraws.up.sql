-- Add up migration script here
CREATE TABLE IF NOT EXISTS withdraws (
    withdraw_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    withdraw_amount INTEGER NOT NULL,
    withdraw_time TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_withdraws_user_id
        FOREIGN KEY(user_id)
        REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);