#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod authentication;
mod competitor;
mod connection;
mod db;
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

embed_migrations!();

#[tokio::main]
async fn main() {
    // TODO: add configuration for postgres db
    let manager = ConnectionManager::<PgConnection>::new(
        "postgres://ai_tournament:docker@localhost:2019/tournament",
    );
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

    authentication::AuthenticationManager::start(
        competitor_tx.clone(),
        game_tx,
        authentication_rx,
        pool.clone(),
    );

    let map = model::Map::new_from_string(include_str!("map.txt"));

    println!("Map has dimensions w{}xh{}", map.width(), map.height());

    game::GlobalManager::start(game_rx, competitor_tx, score_tx, spectator_tx, map);
    score::ScoreManager::start(pool, score_rx);
    // This awaits the server starting only
    spectator::Manager::start("0.0.0.0:3002", spectator_rx).await;

    competitor_handle.await.expect("Competitor handle failed");
}
