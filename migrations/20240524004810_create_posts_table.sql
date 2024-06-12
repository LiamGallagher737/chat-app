-- Add migration script here
CREATE TABLE posts (
    id BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
    user_id BIGINT UNSIGNED NOT NULL,
    content TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id)
);
