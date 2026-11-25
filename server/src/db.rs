use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

use crate::protocol::StashItem;

const STARTING_CASH: i32 = 1000;

#[derive(Debug, Clone)]
pub struct CrewRecord {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub max_hp: f32,
    #[allow(dead_code)]
    pub slot: u32,
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

const STARTER_CREW: &[(&str, &str, f32)] = &[
    ("Vesper", "Point", 100.0),
    ("Rook", "Driver", 100.0),
    ("Iris", "Scout", 90.0),
    ("Hex", "Heavy", 130.0),
];

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
        "SELECT id, name, role, max_hp, slot FROM crew_members WHERE profile_id = ? ORDER BY slot ASC",
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
    for (slot, (name, role, hp)) in STARTER_CREW.iter().enumerate() {
        if existing.contains(&(slot as i64)) {
            continue;
        }
        sqlx::query(
            "INSERT INTO crew_members (id, profile_id, slot, name, role, max_hp) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(profile_id)
        .bind(slot as i64)
        .bind(name)
        .bind(role)
        .bind(*hp as f64)
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
            r#"{"damage":18,"cash_value":0}"#,
        ),
        (
            Uuid::new_v4(),
            "Med",
            "Trauma Kit",
            r#"{"heal":40,"cash_value":0}"#,
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
