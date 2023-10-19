-- migrations/20231019133342_add_salt_to_users.sql
ALTER TABLE users ADD COLUMN salt TEXT NOT NULL;
