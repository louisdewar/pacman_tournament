#[macro_use]
extern crate diesel_migrations;

mod authentication;
mod competitor;
mod connection;
mod game;
mod score;
mod spectator;
mod stream_map_bucket;

pub use stream_map_bucket::StreamMapBucket;

use diesel::{
    r2d2::{self, ConnectionManager},
    PgConnection,
};

pub type PgPool = r2d2::Pool<ConnectionManager<PgConnection>>;

use tokio::sync::mpsc::channel;

embed_migrations!("../db/migrations/");

#[tokio::main]
async fn main() {
    let pg_addr = std::env::var("PG_ADDRESS")
        .unwrap_or("postgres://ai_tournament:docker@localhost:2019/tournament".to_owned());

    println!("Connecting to pg database using {}", pg_addr);

    let manager = ConnectionManager::<PgConnection>::new(&pg_addr);
    let pool = r2d2::Pool::new(manager).unwrap();

    embedded_migrations::run_with_output(&pool.get().unwrap(), &mut std::io::stdout())
        .expect("Failed to run database migrations");

    println!("Database connection ready");

    let (authentication_tx, authentication_rx) = channel(20);
    let (game_tx, game_rx) = channel(20);
    let (competitor_tx, competitor_rx) = channel(20);
    let (score_tx, score_rx) = channel(20);
    let (spectator_tx, spectator_rx) = channel(20);

    let competitor_handle = competitor::Manager::start(
        "0.0.0.0:3001",
        authentication_tx,
        game_tx.clone(),
        competitor_rx,
    )
    .await
    .expect("Couldn't start competitor manager");

    let authentication_handle = authentication::AuthenticationManager::start(
        competitor_tx.clone(),
        game_tx,
        authentication_rx,
        pool.clone(),
    );

    let map = model::Map::new_from_string(include_str!("map.txt"));

    println!("Map has dimensions w{}xh{}", map.width(), map.height());

    let game_handle =
        game::GlobalManager::start(game_rx, competitor_tx, score_tx, spectator_tx, map);
    let score_handle = score::ScoreManager::start(pool.clone(), score_rx);
    // This awaits the server starting only
    let spectator_handle =
        spectator::Manager::start("0.0.0.0:3002", spectator_rx, pool.clone()).await;

    tokio::select! {
        _ = competitor_handle => {}
        _ = authentication_handle => {}
        _ = game_handle => {}
        _ = score_handle => {}
        _ = spectator_handle => {}
    }

    println!("Closing due to crash");
}
