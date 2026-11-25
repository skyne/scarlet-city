CREATE TABLE IF NOT EXISTS profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    player_name TEXT NOT NULL UNIQUE,
    cash INTEGER NOT NULL DEFAULT 1000
);

CREATE TABLE IF NOT EXISTS stash_items (
    id TEXT PRIMARY KEY,
    profile_id INTEGER NOT NULL,
    item_type TEXT NOT NULL,
    item_name TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    stats_json TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);
