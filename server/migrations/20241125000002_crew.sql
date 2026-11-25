CREATE TABLE IF NOT EXISTS crew_members (
    id TEXT PRIMARY KEY,
    profile_id INTEGER NOT NULL,
    slot INTEGER NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    max_hp REAL NOT NULL DEFAULT 100,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS crew_profile_slot ON crew_members(profile_id, slot);
