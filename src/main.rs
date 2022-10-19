use anyhow::Result;
use dotenvy::dotenv;
use retrommo_fetch::prelude::*;
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

    tracing::info!("Petrock Ingest service started successfully.");

    // populate_players_table(pg).await?;

    loop {
        interval.tick().await;

        // Fetch new data.
        let players_online = get_online_players().await?;
        // leaderboard = get_top_players().await?;

        // Calculate experience differences and push to the database.
        // process(&players_online, &leaderboard, pg.clone()).await?;

        tracing::info!("Processed {} players", players_online.len());
        tracing::info!("Online Now: {:?}", players_online);
    }
}

async fn populate_players_table(pg: Database) -> Result<()> {
    let total_player_count = get_registered_player_count().await?;
    let page_count = (total_player_count / 100) as u32;

    tracing::info!("Total player count: {}", total_player_count);
    tracing::info!("Page count: {}", page_count);

    let mut page: i32 = 1;
    let mut rank: i32 = 1;

    for _ in 0..=page_count + 1 {
        tracing::info!("Getting page {}", &page);

        let leaderboard = get_leaderboard_page(Some(page.try_into()?)).await;

        if let Ok(leaderboard_page) = leaderboard {
            for player in leaderboard_page {
                sqlx::query!(
                    "INSERT INTO players (username, experience, rank)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (username) DO UPDATE SET experience = $2, rank = $3",
                    player.username,
                    i32::try_from(player.experience).unwrap_or(0),
                    rank,
                )
                .execute(&*pg)
                .await?;

                rank += 1;
            }

            page += 1;
        }
    }

    Ok(())
}

/// Processes the online list and the leaderboard data.
async fn process(
    players_online: &OnlineList,
    leaderboard: &LeaderboardPage,
    pg: Database,
) -> Result<()> {
    // Update players online, adding them if they are not in the primary table.
    for player in players_online {
        sqlx::query!(
            r#"
            INSERT INTO players (username, online)
            VALUES ($1, true)
            ON CONFLICT (username) DO UPDATE SET online = true
            "#,
            player
        )
        .execute(&*pg)
        .await?;
    }

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
