use tokio::stream::StreamExt;
use tokio::sync::mpsc::Receiver;

use crate::PgPool;

#[derive(Debug, Clone)]
pub struct ScoreUpdate {
    pub game_id: usize,
    pub player_scores: Vec<(usize, u32)>,
}

pub struct ScoreManager {
    rx: Receiver<ScoreUpdate>,
    pool: PgPool,
}

impl ScoreManager {
    pub fn start(pool: PgPool, rx: Receiver<ScoreUpdate>) -> tokio::task::JoinHandle<()> {
        let mut manager = ScoreManager { rx, pool };

        tokio::task::spawn(async move {
            while let Some(update) = manager.rx.next().await {
                manager.handle_score_update(update);
            }
        })
    }

    fn handle_score_update(&self, update: ScoreUpdate) {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool
                .get()
                .expect("Score manager lost connection to the database");
            let user_scores = update
                .player_scores
                .into_iter()
                .map(|(user_id, score)| (user_id as i32, score as i32))
                .collect();

            db::actions::update_scores_if_higher(&conn, user_scores);
        });
    }
}
