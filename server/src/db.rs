use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

use crate::protocol::{CharacterAppearance, StashItem};

const STARTING_CASH: i32 = 1000;

#[derive(Debug, Clone)]
pub struct CrewRecord {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub max_hp: f32,
    #[allow(dead_code)]
    pub slot: u32,
    pub appearance: CharacterAppearance,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: i64,
    #[allow(dead_code)]
    pub player_name: String,
    pub cash: i32,
    pub stash: Vec<StashItem>,
    pub crew: Vec<CrewRecord>,
}

const DEFAULT_APPEARANCE_JSON: &str = r##"{"body_set":"male","hair":"buzzed","outfit":"peasant","beard":false,"brows":true,"hair_color":"#2a1b14","beard_color":"#2a1b14","eye_color":"#6b4a2f","skin_color":"#d8a07a","outfit_color":"#ffffff","height":0.5,"head":0.5,"neck":0.5,"shoulders":0.5,"bust":0.35,"waist":0.5,"hips":0.5,"bum":0.35,"muscle":0.35,"arms":0.5,"legs":0.5,"leg_length":0.5,"torso_length":0.5,"slot_top":"peasant","slot_bottom":"peasant","slot_shoes":"peasant","slot_gloves":"peasant","slot_head":"none","slot_shoulder":"none","slot_belt":"none","weapon":"pistol"}"##;

const STARTER_CREW: &[(&str, &str, f32, &str)] = &[
    (
        "Vesper",
        "Point",
        100.0,
        r##"{"body_set":"female","hair":"buns","outfit":"ranger","beard":false,"brows":true,"hair_color":"#3b2418","beard_color":"#3b2418","eye_color":"#4f7a52","skin_color":"#e0ad86","outfit_color":"#ffffff","height":0.52,"head":0.48,"neck":0.5,"shoulders":0.42,"bust":0.62,"waist":0.42,"hips":0.6,"bum":0.58,"muscle":0.35,"arms":0.4,"legs":0.45,"leg_length":0.52,"torso_length":0.5,"slot_top":"ranger","slot_bottom":"ranger","slot_shoes":"ranger","slot_gloves":"ranger","slot_head":"ranger","slot_shoulder":"ranger","slot_belt":"ranger","weapon":"smg"}"##,
    ),
    (
        "Rook",
        "Driver",
        100.0,
        r##"{"body_set":"male","hair":"buzzed","outfit":"peasant","beard":true,"brows":true,"hair_color":"#1c1512","beard_color":"#1c1512","eye_color":"#5a3a24","skin_color":"#c38a62","outfit_color":"#ffffff","height":0.58,"head":0.52,"neck":0.55,"shoulders":0.78,"bust":0.3,"waist":0.55,"hips":0.45,"bum":0.4,"muscle":0.8,"arms":0.7,"legs":0.6,"leg_length":0.5,"torso_length":0.55,"slot_top":"peasant","slot_bottom":"peasant","slot_shoes":"peasant","slot_gloves":"peasant","slot_head":"none","slot_shoulder":"none","slot_belt":"none","weapon":"pistol"}"##,
    ),
    (
        "Iris",
        "Scout",
        90.0,
        r##"{"body_set":"female","hair":"long","outfit":"peasant","beard":false,"brows":true,"hair_color":"#6b3b1f","beard_color":"#6b3b1f","eye_color":"#3f6fa8","skin_color":"#f0c19c","outfit_color":"#ffffff","height":0.46,"head":0.45,"neck":0.48,"shoulders":0.38,"bust":0.45,"waist":0.36,"hips":0.48,"bum":0.45,"muscle":0.25,"arms":0.35,"legs":0.4,"leg_length":0.48,"torso_length":0.46,"slot_top":"peasant","slot_bottom":"peasant","slot_shoes":"peasant","slot_gloves":"peasant","slot_head":"none","slot_shoulder":"none","slot_belt":"none","weapon":"pistol"}"##,
    ),
    (
        "Hex",
        "Heavy",
        130.0,
        r##"{"body_set":"male","hair":"long","outfit":"ranger","beard":true,"brows":true,"hair_color":"#3a3a3a","beard_color":"#3a3a3a","eye_color":"#b07b32","skin_color":"#a96f4d","outfit_color":"#ffffff","height":0.62,"head":0.55,"neck":0.52,"shoulders":0.85,"bust":0.35,"waist":0.6,"hips":0.5,"bum":0.45,"muscle":0.9,"arms":0.8,"legs":0.7,"leg_length":0.55,"torso_length":0.6,"slot_top":"ranger","slot_bottom":"ranger","slot_shoes":"ranger","slot_gloves":"ranger","slot_head":"ranger","slot_shoulder":"ranger","slot_belt":"ranger","weapon":"rifle"}"##,
    ),
];

pub fn default_appearance() -> CharacterAppearance {
    serde_json::from_str(DEFAULT_APPEARANCE_JSON).expect("valid default appearance")
}

fn merge_json(base: &mut serde_json::Value, patch: serde_json::Value) {
    if let (Some(target), Some(source)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in source {
            merge_json(target.entry(key.clone()).or_insert(serde_json::Value::Null), value.clone());
        }
    } else {
        *base = patch;
    }
}

fn clamp01(value: &mut f32) {
    if !value.is_finite() {
        *value = 0.5;
    }
    *value = value.clamp(0.0, 1.0);
}

pub fn normalize_appearance(appearance: &CharacterAppearance) -> CharacterAppearance {
    let json = serde_json::to_string(appearance).unwrap_or_default();
    appearance_or_default(&json)
}

fn norm_slot(slot: &str, fallback: &str, allowed: &[&str]) -> String {
    if allowed.contains(&slot) { slot.to_string() } else { fallback.to_string() }
}

fn appearance_or_default(raw: &str) -> CharacterAppearance {
    let mut merged = serde_json::to_value(default_appearance()).unwrap_or(serde_json::Value::Null);
    if let Ok(patch) = serde_json::from_str::<serde_json::Value>(raw) {
        merge_json(&mut merged, patch.clone());
        // migrate old wear_* booleans to per-slot outfit sources
        if let Some(obj) = patch.as_object() {
            let outfit = obj.get("outfit").and_then(|v| v.as_str()).unwrap_or("peasant");
            let map = [
                ("wear_top", "slot_top", outfit),
                ("wear_bottom", "slot_bottom", outfit),
                ("wear_shoes", "slot_shoes", outfit),
                ("wear_gloves", "slot_gloves", outfit),
                ("wear_head", "slot_head", if outfit == "ranger" { "ranger" } else { "none" }),
                ("wear_shoulder", "slot_shoulder", if outfit == "ranger" { "ranger" } else { "none" }),
                ("wear_belt", "slot_belt", if outfit == "ranger" { "ranger" } else { "none" }),
            ];
            for (old, newk, on) in map {
                if obj.contains_key(old) && !obj.contains_key(newk) {
                    let v = if obj.get(old).and_then(|v| v.as_bool()).unwrap_or(true) { on } else { "none" };
                    if let Some(m) = merged.as_object_mut() { m.insert(newk.to_string(), serde_json::Value::String(v.to_string())); }
                }
            }
        }
    }
    let mut appearance =
        serde_json::from_value(merged).unwrap_or_else(|_| default_appearance());
    if appearance.hair == "beard" {
        appearance.hair = "buzzed".into();
        appearance.beard = true;
    }
    clamp01(&mut appearance.height);
    clamp01(&mut appearance.head);
    clamp01(&mut appearance.neck);
    clamp01(&mut appearance.shoulders);
    clamp01(&mut appearance.bust);
    clamp01(&mut appearance.waist);
    clamp01(&mut appearance.hips);
    clamp01(&mut appearance.bum);
    clamp01(&mut appearance.muscle);
    clamp01(&mut appearance.arms);
    clamp01(&mut appearance.legs);
    clamp01(&mut appearance.leg_length);
    clamp01(&mut appearance.torso_length);
    let clothing = ["none", "peasant", "ranger", "casual", "suit", "worker", "punk", "tactical"];
    let accessories = ["none", "ranger"];
    appearance.slot_top = norm_slot(&appearance.slot_top, "peasant", &clothing);
    appearance.slot_bottom = norm_slot(&appearance.slot_bottom, "peasant", &clothing);
    appearance.slot_shoes = norm_slot(&appearance.slot_shoes, "peasant", &clothing);
    appearance.slot_gloves = norm_slot(&appearance.slot_gloves, "peasant", &clothing);
    appearance.slot_head = norm_slot(&appearance.slot_head, "none", &accessories);
    appearance.slot_shoulder = norm_slot(&appearance.slot_shoulder, "none", &accessories);
    appearance.slot_belt = norm_slot(&appearance.slot_belt, "none", &accessories);
    let weapons = ["none", "pistol", "revolver", "shotgun", "smg", "rifle", "sniper", "bullpup"];
    appearance.weapon = norm_slot(&appearance.weapon, "pistol", &weapons);
    appearance
}

pub async fn connect(path: &str) -> Result<SqlitePool, sqlx::Error> {
    let url = format!("sqlite://{path}?mode=rwc");
    let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub async fn ensure_profile(pool: &SqlitePool, name: &str) -> Result<Profile, sqlx::Error> {
    let existing: Option<(i64, i64)> =
        sqlx::query_as("SELECT id, cash FROM profiles WHERE player_name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?;

    let profile_id = if let Some((id, _)) = existing {
        id
    } else {
        let res = sqlx::query("INSERT INTO profiles (player_name, cash) VALUES (?, ?)")
            .bind(name)
            .bind(STARTING_CASH)
            .execute(pool)
            .await?;
        let id = res.last_insert_rowid();
        seed_starter_stash(pool, id).await?;
        seed_starter_crew(pool, id).await?;
        id
    };

    ensure_crew(pool, profile_id).await?;
    load_profile(pool, profile_id).await
}

pub async fn load_profile(pool: &SqlitePool, profile_id: i64) -> Result<Profile, sqlx::Error> {
    let row = sqlx::query("SELECT id, player_name, cash FROM profiles WHERE id = ?")
        .bind(profile_id)
        .fetch_one(pool)
        .await?;
    let stash = load_stash(pool, profile_id).await?;
    let crew = load_crew(pool, profile_id).await?;
    Ok(Profile {
        id: row.get("id"),
        player_name: row.get("player_name"),
        cash: row.get::<i64, _>("cash") as i32,
        stash,
        crew,
    })
}

pub async fn load_crew(pool: &SqlitePool, profile_id: i64) -> Result<Vec<CrewRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, role, max_hp, slot, appearance_json FROM crew_members WHERE profile_id = ? ORDER BY slot ASC",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| CrewRecord {
            id: Uuid::parse_str(r.get::<String, _>("id").as_str()).unwrap_or_else(|_| Uuid::new_v4()),
            name: r.get("name"),
            role: r.get("role"),
            max_hp: r.get::<f64, _>("max_hp") as f32,
            slot: r.get::<i64, _>("slot").max(0) as u32,
            appearance: appearance_or_default(r.get("appearance_json")),
        })
        .collect())
}

pub async fn load_stash(pool: &SqlitePool, profile_id: i64) -> Result<Vec<StashItem>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, item_type, item_name, quantity, stats_json FROM stash_items WHERE profile_id = ?",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| StashItem {
            id: Uuid::parse_str(r.get::<String, _>("id").as_str()).unwrap_or_else(|_| Uuid::new_v4()),
            item_type: r.get("item_type"),
            item_name: r.get("item_name"),
            quantity: r.get::<i64, _>("quantity").max(0) as u32,
            stats_json: r.get("stats_json"),
        })
        .collect())
}

pub async fn take_loadout(
    pool: &SqlitePool,
    profile_id: i64,
    ids: &[Uuid],
) -> Result<Vec<StashItem>, sqlx::Error> {
    let mut taken = Vec::new();
    for id in ids {
        let key = id.to_string();
        let row = sqlx::query(
            "SELECT id, item_type, item_name, quantity, stats_json FROM stash_items WHERE id = ? AND profile_id = ?",
        )
        .bind(&key)
        .bind(profile_id)
        .fetch_optional(pool)
        .await?;
        if let Some(r) = row {
            sqlx::query("DELETE FROM stash_items WHERE id = ?")
                .bind(&key)
                .execute(pool)
                .await?;
            taken.push(StashItem {
                id: *id,
                item_type: r.get("item_type"),
                item_name: r.get("item_name"),
                quantity: r.get::<i64, _>("quantity").max(0) as u32,
                stats_json: r.get("stats_json"),
            });
        }
    }
    Ok(taken)
}

pub async fn insert_items(
    pool: &SqlitePool,
    profile_id: i64,
    items: &[StashItem],
) -> Result<(), sqlx::Error> {
    for item in items {
        sqlx::query(
            "INSERT INTO stash_items (id, profile_id, item_type, item_name, quantity, stats_json) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(profile_id)
        .bind(&item.item_type)
        .bind(&item.item_name)
        .bind(item.quantity as i64)
        .bind(&item.stats_json)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn add_cash(pool: &SqlitePool, profile_id: i64, delta: i32) -> Result<i32, sqlx::Error> {
    sqlx::query("UPDATE profiles SET cash = cash + ? WHERE id = ?")
        .bind(delta)
        .bind(profile_id)
        .execute(pool)
        .await?;
    let cash: i64 = sqlx::query_scalar("SELECT cash FROM profiles WHERE id = ?")
        .bind(profile_id)
        .fetch_one(pool)
        .await?;
    Ok(cash as i32)
}

pub async fn update_crew_appearance(
    pool: &SqlitePool,
    profile_id: i64,
    crew_id: Uuid,
    appearance: &CharacterAppearance,
) -> Result<bool, sqlx::Error> {
    let json = serde_json::to_string(appearance).unwrap_or_else(|_| "{}".into());
    let result = sqlx::query(
        "UPDATE crew_members SET appearance_json = ? WHERE id = ? AND profile_id = ?",
    )
    .bind(json)
    .bind(crew_id.to_string())
    .bind(profile_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_crew_name(
    pool: &SqlitePool,
    profile_id: i64,
    crew_id: Uuid,
    name: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE crew_members SET name = ? WHERE id = ? AND profile_id = ?")
        .bind(name)
        .bind(crew_id.to_string())
        .bind(profile_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

async fn ensure_crew(pool: &SqlitePool, profile_id: i64) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crew_members WHERE profile_id = ?")
        .bind(profile_id)
        .fetch_one(pool)
        .await?;
    if count >= 4 {
        return Ok(());
    }
    seed_starter_crew(pool, profile_id).await
}

async fn seed_starter_crew(pool: &SqlitePool, profile_id: i64) -> Result<(), sqlx::Error> {
    let existing: Vec<i64> = sqlx::query_scalar("SELECT slot FROM crew_members WHERE profile_id = ?")
        .bind(profile_id)
        .fetch_all(pool)
        .await?;
    for (slot, (name, role, hp, appearance)) in STARTER_CREW.iter().enumerate() {
        if existing.contains(&(slot as i64)) {
            continue;
        }
        sqlx::query(
            "INSERT INTO crew_members (id, profile_id, slot, name, role, max_hp, appearance_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(profile_id)
        .bind(slot as i64)
        .bind(name)
        .bind(role)
        .bind(*hp as f64)
        .bind(*appearance)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_starter_stash(pool: &SqlitePool, profile_id: i64) -> Result<(), sqlx::Error> {
    let starters = [
        (
            Uuid::new_v4(),
            "Weapon",
            "Street Pistol",
            r##"{"damage":18,"cash_value":0}"##,
        ),
        (
            Uuid::new_v4(),
            "Med",
            "Trauma Kit",
            r##"{"heal":40,"cash_value":0}"##,
        ),
    ];
    for (id, ty, name, stats) in starters {
        sqlx::query(
            "INSERT INTO stash_items (id, profile_id, item_type, item_name, quantity, stats_json) VALUES (?, ?, ?, ?, 1, ?)",
        )
        .bind(id.to_string())
        .bind(profile_id)
        .bind(ty)
        .bind(name)
        .bind(stats)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn loot_cash_value(item: &StashItem) -> i32 {
    if item.item_type != "Loot" {
        return 0;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&item.stats_json) {
        return v.get("cash_value").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    }
    0
}
