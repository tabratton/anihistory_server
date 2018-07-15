CREATE Table anime (
  anime_id INT PRIMARY KEY,
  description TEXT NOT NULL,
  cover_s3 TEXT NOT NULL,
  cover_anilist TEXT NOT NULL,
  average SMALLINT,
  native TEXT,
  romaji TEXT,
  english TEXT
)