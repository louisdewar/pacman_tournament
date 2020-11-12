use tokio::stream::StreamExt;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::competitor::IncomingEvent as CompetitorIncomingEvent;
use crate::competitor::ManagerEvent as CompetitorManagerEvent;
use crate::PgPool;

#[derive(Clone, Debug)]
pub enum AuthenticationEvent {
    Authenticated {
        temporary_id: usize,
        id: usize,
    },
    BadAuthentication {
        temporary_id: usize,
        reason: AuthenticationFailedReason,
    },
}

#[derive(Clone, Debug)]
pub enum AuthenticationFailedReason {
    PlayerInGame,
    BadCode,
    PlayerNotFound,
    PlayerNotEnabled,
}

impl AuthenticationFailedReason {
    pub fn to_message(self) -> String {
        use AuthenticationFailedReason::*;

        match self {
            PlayerInGame => "You are already in a game, or waiting to spawn in one".to_string(),
            BadCode => "The code does not match your username".to_string(),
            PlayerNotFound => "Your username does not exist".to_string(),
            PlayerNotEnabled => "Your account is not enabled".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthenticationRequest {
    pub username: String,
    pub code: String,
    pub temporary_id: usize,
}

pub struct AuthenticationManager {
    competitor_tx: Sender<CompetitorIncomingEvent>,
    game_tx: Sender<CompetitorManagerEvent>,
    authentication_rx: Receiver<AuthenticationRequest>,
    db_pool: PgPool,
}

impl AuthenticationManager {
    pub fn start(
        competitor_tx: Sender<CompetitorIncomingEvent>,
        game_tx: Sender<CompetitorManagerEvent>,
        authentication_rx: Receiver<AuthenticationRequest>,
        db_pool: PgPool,
    ) -> tokio::task::JoinHandle<()> {
        let mut manager = AuthenticationManager {
            competitor_tx,
            game_tx,
            authentication_rx,
            db_pool,
        };

        tokio::task::spawn(async move {
            while let Some(request) = manager.authentication_rx.next().await {
                manager.authenticate_user(request);
            }
        })
    }

    /// This functions exists immediately but spawns a thread to handle the blocking database call.
    /// It will clone the channels so it can send back messages by itself.
    fn authenticate_user(&self, request: AuthenticationRequest) {
        let game_tx = self.game_tx.clone();
        let competitor_tx = self.competitor_tx.clone();
        let pool = self.db_pool.clone();
        // We spawn a task to avoid blocking the rest of authentication manager but we don't
        // directly use spawn_blocking because we want to use async when sending the message.
        tokio::task::spawn(async move {
            let connection = pool.get().expect("Lost connection to the database");
            let username = request.username.clone();
            match tokio::task::spawn_blocking(move || {
                db::actions::get_user_by_username(&connection, &username)
            })
            .await
            .unwrap()
            {
                Ok(Some(user)) => {
                    if user.code != request.code {
                        competitor_tx
                            .send(CompetitorIncomingEvent::Authentication(
                                AuthenticationEvent::BadAuthentication {
                                    temporary_id: request.temporary_id,
                                    reason: AuthenticationFailedReason::BadCode,
                                },
                            ))
                            .await
                            .unwrap();
                        return;
                    }

                    if !user.enabled {
                        competitor_tx
                            .send(CompetitorIncomingEvent::Authentication(
                                AuthenticationEvent::BadAuthentication {
                                    temporary_id: request.temporary_id,
                                    reason: AuthenticationFailedReason::PlayerNotEnabled,
                                },
                            ))
                            .await
                            .unwrap();
                        return;
                    }

                    game_tx
                        .send(CompetitorManagerEvent::Authenticated {
                            username: user.username,
                            high_score: user.high_score as u32,
                            user_id: user.id as usize,
                            temporary_id: request.temporary_id,
                        })
                        .await
                        .unwrap();
                }
                Ok(None) => {
                    println!("Username {} doesn't exist", request.username);
                    competitor_tx
                        .send(CompetitorIncomingEvent::Authentication(
                            AuthenticationEvent::BadAuthentication {
                                temporary_id: request.temporary_id,
                                reason: AuthenticationFailedReason::PlayerNotFound,
                            },
                        ))
                        .await
                        .unwrap();
                }
                Err(_) => println!("Error connecting to the database"),
            };
        });
    }
}
