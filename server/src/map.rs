use std::path::Path;

use crate::protocol::MapData;

pub fn load_map(path: impl AsRef<Path>) -> MapData {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read map {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid map JSON {}: {e}", path.display()))
}

pub fn default_map_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("SCARLET_MAP") {
        return p.into();
    }
    let candidates = [
        std::path::PathBuf::from("../shared/maps/map_01.json"),
        std::path::PathBuf::from("shared/maps/map_01.json"),
        std::path::PathBuf::from("maps/map_01.json"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    std::path::PathBuf::from("../shared/maps/map_01.json")
}
