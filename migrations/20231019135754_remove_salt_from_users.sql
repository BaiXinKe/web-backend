-- migrations/20231019135754_remove_salt_from_users.sql
ALTER TABLE users DROP COLUMN salt;
