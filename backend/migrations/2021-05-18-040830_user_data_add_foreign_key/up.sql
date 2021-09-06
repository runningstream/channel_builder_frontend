ALTER TABLE user_data ADD CONSTRAINT active_channel_fk FOREIGN KEY (active_channel) REFERENCES channel_list (id) ON DELETE SET NULL
--ALTER TABLE IF EXISTS NOTHING VALIDATE CONSTRAINT active_channel_fk
