use std::env;
use std::io::{BufRead, stdin};
use std::sync::mpsc::channel;
use crate::handler::RequestHandler;
use crate::listener::Listener;
use crate::persistence::InMemoryDbClient;

mod handler;
mod listener;
mod persistence;

/// Listens for incoming packets until the user exits the program.
/// Expects two env arguments: <program name, port>.
pub fn main() {
    let args = env::args().collect::<Vec<String>>();
    let address = check_args_and_get_address(&args);

    let (db_sender, _) = channel::<String>();
    let db_client = InMemoryDbClient::new(db_sender);
    let handler = RequestHandler::new(db_client);

    let listener = Listener::new(handler);
    listener.listen(address.to_string());

    loop_until_exit_requested(stdin().lock());

    listener.stop_listening();
}

/// Returns a localhost address based on the port provided. Expects arguments of the form "<program_name> <port>".
fn check_args_and_get_address(args: &[String]) -> String {
    let port = match args.len() {
        0 => panic!("Too few arguments. Usage is '<program_name> <port>'."),
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

/// Loop until the reader returns the line 'exit' (plus optional whitespace).
fn loop_until_exit_requested<R: BufRead>(mut reader: R) -> String {
    loop {
        println!("Type 'exit' to exit.");
        let mut maybe_exit = String::new();
        
        reader.read_line(&mut maybe_exit).expect("Failed to read line.");
        if maybe_exit.trim() == "exit" {
            return maybe_exit.trim().to_string();
        }
    }
}