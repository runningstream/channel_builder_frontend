CREATE TABLE user_data (
    id SERIAL PRIMARY KEY,
    username VARCHAR(321) UNIQUE NOT NULL,
    pass_hash VARCHAR NOT NULL,
    pass_hash_type INTEGER NOT NULL,
    validation_status BOOLEAN NOT NULL,
    validation_code VARCHAR,
    active_channel INTEGER
)
