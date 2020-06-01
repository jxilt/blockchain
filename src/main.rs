use std::env;
use std::io::{BufRead, stdin,};
use std::sync::mpsc::{channel, Receiver};
use crate::handler::FlowSessionHandler;
use crate::listener::Listener;
use crate::persistence::InMemoryDbClient;

mod handler;
mod listener;
mod persistence;

/// Creates a listener and loops until the user breaks.
/// 
/// Expects two env arguments, with the port in second position.
pub fn main() {
    let args = env::args().collect::<Vec<String>>();
    let address = get_address(&args);

    let (db_sender, db_receiver) = channel::<String>();
    let db_client = InMemoryDbClient::new(db_sender);
    let handler = FlowSessionHandler::new(db_client);

    let listener = Listener::new(address.to_string(), handler);

    loop_until_user_types_exit(stdin().lock());

    echo_all_valid_packets(db_receiver);

    listener.stop_listening();
}

/// Creates the address based on the port passed in on the command line.
/// 
/// Expects two arguments, with the port in second position.
fn get_address(args: &[String]) -> String {
    let port = match args.len() {
        0 => panic!("Too few arguments. Usage is '<program_name> <port>."),
        1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            default_port
        },
        2 => {
            let provided_port = &args[1];
            println!("Using provided port '{}'.", provided_port);
            provided_port
        },
        _ => panic!("Too many arguments. Usage is '<program_name> <port>.")
    };

    return format!("localhost:{}", port);
}

/// Loop until the user types 'exit'.
fn loop_until_user_types_exit<R: BufRead>(mut reader: R) -> String {
    loop {
        println!("Type 'exit' to exit.");
        let mut maybe_exit = String::new();
        
        reader.read_line(&mut maybe_exit).expect("Failed to read line.");
        if maybe_exit.trim() == "exit" {
            return maybe_exit.trim().to_string();
        }
    }
}

/// Echoes the contents of valid packets received while the listener was running.
fn echo_all_valid_packets(db_receiver: Receiver<String>) {
    loop {
        let received_value = db_receiver.try_recv();
        match received_value {
            // TODO: Better error handling - distinguish by error.
            Ok(contents) => println!("{}", contents),
            Err(_) => break
        };
    }
}