use tokio;
use tokio::task::{JoinHandle, spawn_blocking};
use std::sync::mpsc::Receiver;

use chess::model::game_state;
use chess::model::game_state::{Color, GameState};
use chess::model::move_generator::MoveGenerator;
use chess::search::minimax_search::negamax_alpha_beta_with_trasposition_table;
use chess::search::transposition_table::TranspositionTable;
use chess::uci::uci_utils;

use crate::client::{LichessClient, Challenge, Game, LichessProfile, LichessError, GameEvent};


pub async fn handle_challenge(client: LichessClient, challenge: Challenge) -> Result<(), LichessError> {
    if challenge.variant.key != "standard" || challenge.rated == true {
        return Ok(());
    }

    client.accept_challenge(challenge.id).await   
}

pub async fn spawn_game_handler(client: LichessClient, game: Game, profile: LichessProfile) -> JoinHandle<()> {
    return spawn_blocking(move || {
        match client.game_stream(game.id) {
            Ok(stream) => {
                if let Err(err) = play_game(client, profile, stream) {
                    println!("Error while executing game: {:?}", err);
                }
            },
            Err(err) => {
                println!("Failed to read game events");
            }
        };
    });
}

fn play_game(client: LichessClient, profile: LichessProfile, events: Receiver<GameEvent>) -> Result<(), String> {
    println!("---game starting---");

    let initial_event = events.recv().map_err(|err| err.to_string())?;
    let game_id = initial_event.id.unwrap();

    let ai_color = if initial_event.white.unwrap().id == profile.id {
        Color::WHITE
    } else {
        Color::BLACK
    };

    println!("AI plays as {:?}", ai_color);

    let mut game_state = game_state::GameState::new();
    let move_generator = MoveGenerator::new();
    let mut transposition_table = TranspositionTable::with_capacity(500_000);

    let mut turn = 1;

    loop {
        if game_state.to_move() == ai_color {
            apply_ai_move(&client, &game_id, &move_generator, &mut game_state, &mut transposition_table);
            turn += 1;
        } else {
            apply_opponent_move(&events, &move_generator, &mut game_state, &turn);
            turn += 1;
        }
    }

    Ok(())
}

fn apply_ai_move(
    client: &LichessClient,
    game_id: &String,
    move_generator: &MoveGenerator,
    game_state: &mut GameState,
    table: &mut TranspositionTable) -> Result<(), LichessError> {

    let (next_move, _, _) = negamax_alpha_beta_with_trasposition_table(game_state, move_generator, table, 5);
    let uci_move = uci_utils::move_to_uci(&next_move.unwrap());

    println!("AI plays: {}", uci_move);

    let client_clone = client.clone();
    let game_id_clone = game_id.clone();

    game_state.apply_move_mut(next_move.unwrap());

    tokio::spawn(async move {
        println!("Sending move request...");
        if let Err(e) = client_clone.play_move(game_id_clone, uci_move.to_string()).await {
            println!("Failed to play move: {:?}", e);
        };
    });

    Ok(())
}

fn apply_opponent_move(events: &Receiver<GameEvent>, move_generator: &MoveGenerator, game_state: &mut GameState, turn: &u32) -> Result<(), String> {
    let last_move = loop {
        let event = events.recv().map_err(|err| err.to_string())?;
        let moves_raw = event.moves.unwrap();
        let all_moves = moves_raw.split(" ").collect::<Vec<&str>>();
        
        if all_moves.len() >= (*turn as usize) { 
            break all_moves
                .into_iter()
                .nth((*turn as usize) - 1)
                .unwrap()
                .to_string() 
            }
    };

    let uci_move = uci_utils::parse_move(last_move.as_str())?;
    
    let next_move = move_generator.generate_moves(game_state)
        .moves
        .into_iter()
        .find(|m| m.from == uci_move.0 && m.to == uci_move.1 && m.promotes_to == uci_move.2)
        .unwrap();

    println!("Opponent played: {}", uci_move);        
    
    game_state.apply_move_mut(next_move);

    Ok(())
}