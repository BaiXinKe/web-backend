-- migrations/20231019130130_rename_password_column.sql
ALTER TABLE users RENAME password TO password_hash;