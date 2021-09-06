--ALTER TABLE user_data DROP CONSTRAINT active_channel_fk
ALTER TABLE user_data DROP CONSTRAINT IF EXISTS active_channel_fk
