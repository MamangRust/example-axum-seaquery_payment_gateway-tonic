-- Add up migration script here
CREATE TABLE IF NOT EXISTS "saldo" (
    saldo_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    total_balance INTEGER NOT NULL,
    withdraw_amount INTEGER DEFAULT 0,
    withdraw_time TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_saldo_user_id
        FOREIGN KEY(user_id)
        REFERENCES users(user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);