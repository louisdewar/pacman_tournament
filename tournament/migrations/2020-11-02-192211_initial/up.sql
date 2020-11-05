CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  code TEXT NOT NULL,
  high_score INT NOT NULL default 0,
  live boolean NOT NULL DEFAULT false
)
