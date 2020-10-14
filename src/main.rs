use std::env;
use std::env::Args;
use std::io::{BufRead, stdin};

use crate::server::Server;
use crate::servererror::{Result, ServerError};

mod handler;
mod persistence;
mod server;
mod servererror;
mod serverinternal;

const DEFAULT_PORT: &str = "10005";

/// Listens for incoming packets until the user exits the program.
/// Expects two env arguments: <program name, port>.
pub fn main() {
    let args = env::args();
    let port = extract_port_from_args(args)
        .expect("Could not parse port, or flag '-p' provided without corresponding port.");
    let address = format!("0.0.0.0:{}", port);

    let routes = [
        ("/".to_string(), "./src/hello_world.html".to_string())
    ].iter().cloned().collect();
    let mut server = Server::new(routes);

    server.listen(&address).expect("Server could not listen on address.");
    loop_until_exit_requested(stdin().lock()).expect("Failed to read input.");
    server.stop_listening().expect("Server could not stop listening.");
}

/// Returns a localhost address based on the port provided using the '-p' flag.
fn extract_port_from_args(mut args: Args) -> Result<String> {
    loop {
        match args.next() {
            None => {
                println!("No port provided. Using default of '{}'.", DEFAULT_PORT);
                return Ok(DEFAULT_PORT.to_string());
            },
            Some(maybe_port_flag) => {
                if maybe_port_flag == "-p".to_string() {
                    break;
                }
            }
        }
    }

    // The port should be the argument following the '-p' flag.
    let port = args.next()
        .ok_or(ServerError { message: "No argument passed after '-p' flag.".to_string() })?;

    port.parse::<i32>().map_err(|_e| ServerError { message: "Could not parse port value.".to_string() })?;
    println!("Using provided port of {}.", port);
    return Ok(port);
}

/// Loop until the reader reads the word 'exit' (plus optional whitespace).
fn loop_until_exit_requested<R: BufRead>(mut reader: R) -> Result<()> {
    let mut maybe_exit = String::new();

    loop {
        println!("Type 'exit' to exit.");
        maybe_exit.clear();

        reader.read_line(&mut maybe_exit)
            .map_err(|_e| ServerError { message: "Could not read from stream.".to_string() })?;

        if maybe_exit.trim() == "exit" {
            return Ok(());
        }
    }
}