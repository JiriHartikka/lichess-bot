mod client;
mod game_manager;

extern crate tokio;

extern crate chess;

use std::env;
use std::process;

use client::LichessClient;
use crate::game_manager::{handle_challenge, spawn_game_handler};

#[tokio::main]
pub async fn main() {

    let access_token = match env::var("LICHESS_ACCESS_TOKEN") {
        Ok(token) if token != "" => token,
        _ => {
            println!("Set environment variable LICHESS_ACCESS_TOKEN");
            process::exit(2);
        } 
    };

    let client = LichessClient::new(access_token);

    // Panic if profile cannot be fetched
    let profile = client.get_profile().await.unwrap();

    println!("Bot id is {}", profile.id);

    let event_stream = match client.event_stream() {
        Ok(receiver) => receiver,
        Err(e) => {
            println!("Failed to open event stream");
            println!("Error: {}", e);
            process::exit(1);
        }
    };

    while let Ok(event) = event_stream.recv() {
        if event.event_type == "challenge" {
            let client_clone = client.clone();
            match handle_challenge(client_clone, event.challenge.unwrap()).await {
                Ok(_) => (),
                Err(err) => println!("Error: {:?}", err),
            }
        }
        else if event.event_type == "gameStart" {
            let client_clone = client.clone();
            let _handle = spawn_game_handler(client_clone, event.game.unwrap(),  profile.clone()).await;
        }

    } 
    
}
