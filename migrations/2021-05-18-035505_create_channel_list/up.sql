CREATE TABLE channel_list (
    id SERIAL PRIMARY KEY,
    userid INTEGER NOT NULL REFERENCES user_data (id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    data VARCHAR NOT NULL DEFAULT '{"entries": []}'
)
