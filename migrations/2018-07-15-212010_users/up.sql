CREATE TABLE users (
    user_id INT PRIMARY KEY,
    name TEXT NOT NULL,
    avatar_s3 TEXT NOT NULL,
    avatar_anilist TEXT NOT NULL
)

CREATE INDEX idx_name ON users(name);
