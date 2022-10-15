use anyhow::Result;
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::time::Duration;

use std::{env, sync::Arc};

const API_URL_PLAYERS: &str = "https://play.retro-mmo.com/players.json";
const API_URL_LEADERBOARD: &str = "https://play.retro-mmo.com/leaderboards.json";

type Database = Arc<PgPool>;
type OnlineList = Vec<String>;
type Leaderboard = Vec<LeaderboardEntry>;

#[derive(Debug, Deserialize, Serialize)]
struct LeaderboardEntry {
    experience: u64, // Should never be negative. I assume Evan checks.
    permissions: u8, // Should also be safe...
    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let pg = Arc::new(create_pool().await?);
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    let mut leaderboard: Leaderboard = Vec::new();
    let mut players_online: OnlineList = Vec::new();

    tracing::info!("Petrock Ingest service started successfully.");

    loop {
        interval.tick().await;
        fetch_players(&mut players_online).await?;
        fetch_leaderboard(&mut leaderboard).await?;
        process(&players_online, &leaderboard, pg.clone()).await?;

        tracing::info!("Processed {} players", players_online.len());
        tracing::info!("Online Now: {:?}", players_online);
    }
}

async fn fetch_players(players_online: &mut OnlineList) -> Result<()> {
    let response = reqwest::get(API_URL_PLAYERS).await?;

    let mut json = response.json::<OnlineList>().await?;
    std::mem::swap(players_online, &mut json);

    Ok(())
}

async fn fetch_leaderboard(leaderboard: &mut Leaderboard) -> Result<()> {
    let response = reqwest::get(API_URL_LEADERBOARD).await?;

    let mut json = response.json::<Leaderboard>().await?;
    std::mem::swap(leaderboard, &mut json);

    Ok(())
}

/// Processes the online list and the leaderboard data.
async fn process(
    _players_online: &OnlineList,
    _leaderboard: &Leaderboard,
    _pg: Database,
) -> Result<()> {
    // sqlx::query!(
    //     r#"
    //     INSERT INTO players (username, experience, rank, online)
    //     VALUES ($1, $2, $3, $4)
    //     ON CONFLICT ON CONSTRAINT players_pkey
    //     DO UPDATE
    //     SET
    //         experience = $2,
    //         rank = $3,
    //         online = $4
    //     WHERE players.username = $1;
    //     "#,
    //     ,
    // )
    // .execute(pg)
    // .await?;

    // sqlx::query!(
    //     r#"
    //     INSERT INTO history (player, experience)
    //     VALUES ($1, $2)
    //     ON CONFLICT (player, created_on)
    //     DO UPDATE
    //     SET
    //         experience = COALESCE(history.experience + $2)
    //     WHERE history.player = $1;
    //     )"#,
    // )
    // .execute(pg)
    // .await?;

    Ok(())
}

/// Attempts to create a connection pool to the database specified in
/// environment variables. Panics if it cannot connect.
async fn create_pool() -> Result<PgPool> {
    //! FIXME: Should save data in memory if we can't connect, and try
    //! intermittently during the main loop.
    let pool = PgPool::connect_with({
        PgConnectOptions::new()
            .host(env::var("PGHOST").unwrap_or_else(|_| "localhost".into()).as_str())
            .port(
                env::var("PGPORT")
                    .unwrap_or_else(|_| "5432".into())
                    .parse::<u16>()
                    .unwrap_or_default(),
            )
            .username(env::var("PGUSER").unwrap_or_else(|_| "postgres".into()).as_str())
            .password(env::var("PGPASS").unwrap_or_else(|_| "postgres".into()).as_str())
            .database(env::var("PGDBNAME").unwrap_or_else(|_| "postgres".into()).as_str())
    })
    .await
    .expect("Failed to connect to PostgreSQL database.");

    Ok(pool)
}
