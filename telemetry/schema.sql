CREATE TABLE IF NOT EXISTS installations (id TEXT PRIMARY KEY, version TEXT NOT NULL, platform TEXT NOT NULL, last_seen INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS installations_last_seen_idx ON installations(last_seen);
