-- Add migration script here
CREATE TABLE users (
    id BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
    username VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL
);
