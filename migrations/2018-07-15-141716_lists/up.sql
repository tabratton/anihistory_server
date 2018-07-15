CREATE TABLE lists (
  user_id INT REFERENCES users (user_id),
  anime_id INT REFERENCES anime (anime_id),
  user_title TEXT,
  start_day DATE,
  end_day DATE,
  score SMALLINT,
  PRIMARY KEY (user_id, anime_id)
)