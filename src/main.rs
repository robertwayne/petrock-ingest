use anyhow::Result;
use dotenvy::dotenv;
use retrommo_fetch::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::time::Duration;

use std::{env, sync::Arc};

type Database = Arc<PgPool>;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let pg = Arc::new(create_pool().await?);
    let mut interval = tokio::time::interval(Duration::from_secs(
        env::var("TICK_RATE")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .expect("Environment variables TICK_RATE not set."),
    ));

    let mut leaderboard: LeaderboardPage = Vec::new();
    let mut players_online: OnlineList = Vec::new();

    tracing::info!("Petrock Ingest service started successfully.");

    loop {
        interval.tick().await;

        // Fetch new data.
        players_online = get_online_players().await?;
        leaderboard = get_top_players().await?;

        // Calculate experience differences and push to the database.
        process(&players_online, &leaderboard, pg.clone()).await?;

        tracing::info!("Processed {} players", players_online.len());
        tracing::info!("Online Now: {:?}", players_online);
    }
}

/// Processes the online list and the leaderboard data.
async fn process(
    _players_online: &OnlineList,
    _leaderboard: &LeaderboardPage,
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
    let url = env::var("DATABASE_URL").expect("Environment variables DATABASE_URL not set.");
    let pool =
        PgPool::connect(url.as_str()).await.expect("Failed to connect to PostgreSQL database.");

    Ok(pool)
}
