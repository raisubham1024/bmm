ALTER TABLE bookmarks ADD COLUMN starred INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_starred ON bookmarks (starred);
