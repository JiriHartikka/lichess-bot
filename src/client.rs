extern crate reqwest;

extern crate bytes;
extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};

use std::ops::Deref;
use std::str::from_utf8;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

use bytes::Bytes;

use futures::stream::StreamExt;

use reqwest::Error;

#[derive(Clone)]
pub struct LichessClient {
    access_token: String,
}

impl LichessClient {
    pub fn new(access_token: String) -> Self {
        LichessClient { access_token }
    }

    pub fn event_stream(&self) -> Result<Receiver<LichessEvent>, Error> {
        let (sender, receiver) = channel();

        let client = reqwest::Client::new();

        let response = client
            .get("https://lichess.org/api/stream/event")
            .header("authorization", format!("Bearer {}", self.access_token))
            .send();

        let _handle = tokio::spawn(async move {
            match response.await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(bytes) => {
                                //println!("Event raw: {:?}", bytes);

                                match parse_json(&bytes) {
                                    Ok(Some(event)) => {
                                        if let Err(err) = sender.send(event) {
                                            println!("Error sending bot event: {}", err);
                                            break;
                                        }
                                    }
                                    Ok(None) => continue,
                                    Err(err) => {
                                        println!("Error while parsing event: {:?}", err);
                                        break;
                                    }
                                }
                            }
                            Err(err) => {
                                println!("Error while receiving events: {}", err.to_string());
                                break;
                            }
                        };
                    }
                }
                Err(e) => {
                    println!("Error while opening event stream: {}", e.to_string());
                }
            }
        });

        Ok(receiver)
    }

    pub fn game_stream(&self, game_id: String) -> Result<Receiver<GameEvent>, Error> {
        println!("game stream starting...");

        let (sender, receiver) = channel();

        let client = reqwest::Client::new();

        let response = client
            .get(&format!(
                "https://lichess.org/api/bot/game/stream/{}",
                game_id
            ))
            .header("authorization", format!("Bearer {}", self.access_token))
            .send();

        println!("stream request sent...");

        let _handle = tokio::spawn(async move {
            println!("task spawned");

            match response.await {
                Ok(response) => {
                    println!("opening game stream");
                    let mut stream = response.bytes_stream();

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(bytes) => {
                                //println!("Game event raw: {:?}", bytes);

                                match parse_json(&bytes) {
                                    Ok(Some(event)) => {
                                        if let Err(err) = sender.send(event) {
                                            println!("Error sending game event: {}", err);
                                            break;
                                        }
                                    }
                                    Ok(None) => continue,
                                    Err(err) => {
                                        println!("Error while parsing event: {:?}", err);
                                        break;
                                    }
                                };
                            }
                            Err(err) => {
                                println!("Error while receiving game events: {}", err.to_string());
                                break;
                            }
                        };
                    }
                }
                Err(e) => {
                    println!("Error while opening event stream: {}", e.to_string());
                }
            }
        });

        Ok(receiver)
    }

    pub async fn accept_challenge(&self, challenge_id: String) -> Result<(), LichessError> {
        println!("Accepting challenge");

        reqwest::Client::new()
            .post(&format!(
                "https://lichess.org/api/challenge/{}/accept",
                challenge_id
            ))
            .header("authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| LichessError::RequestError(format!("{:?}", e)))
            .map(|_| ())
    }

    pub async fn play_move(&self, game_id: String, uci_move: String) -> Result<(), LichessError> {
        reqwest::Client::new()
            .post(&format!(
                "https://lichess.org/api/bot/game/{}/move/{}",
                game_id, uci_move
            ))
            .header("authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| LichessError::RequestError(format!("{:?}", e)))
            .map(|_| ())
    }

    pub async fn get_profile(&self) -> Result<LichessProfile, LichessError> {
        // todo: sensible conversion from client::reqwest::Error to domain error LichessError
        reqwest::Client::new()
            .get("https://lichess.org/api/account")
            .header("authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| LichessError::RequestError(format!("{:?}", e)))
            .map(|response| response.bytes())?
            .await
            .map_err(|e| LichessError::RequestError(format!("{:?}", e)))
            .and_then(|response_body| {
                parse_json(&response_body)?.ok_or(LichessError::ParseError(
                    "Could not parse LichessProfile".to_string(),
                ))
            })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LichessEvent {
    #[serde(rename(deserialize = "type"))]
    pub event_type: String,
    pub challenge: Option<Challenge>,
    pub game: Option<Game>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Challenge {
    pub id: String,
    pub variant: Variant,
    pub rated: bool,
    pub speed: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Game {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Variant {
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LichessProfile {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GameEvent {
    #[serde(rename(deserialize = "type"))]
    pub event_type: String,
    pub id: Option<String>,
    pub white: Option<Player>,
    pub black: Option<Player>,
    pub moves: Option<String>,
    pub wtime: Option<i32>,
    pub btime: Option<i32>,
    pub winc: Option<i32>,
    pub binc: Option<i32>,
    pub wdraw: Option<bool>,
    pub bdraw: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub rating: i32,
}

#[derive(Debug)]
pub enum LichessError {
    RequestError(String),
    ParseError(String),
}

fn parse_json<'a, T: Deserialize<'a>>(event_bytes: &'a Bytes) -> Result<Option<T>, LichessError> {
    let event_str =
        from_utf8(event_bytes.deref()).map_err(|e| LichessError::ParseError(e.to_string()))?;

    if event_str == "\n" {
        return Ok(None);
    }

    serde_json::from_str(event_str)
        .map_err(|e| LichessError::ParseError(e.to_string()))
        .map(|event| Some(event))
}
