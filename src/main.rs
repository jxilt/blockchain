use std::env;
use std::env::Args;
use std::io::{BufRead, stdin};
use crate::server::Server;

mod server;
mod serverinternal;
mod handler;
mod persistence;

/// Listens for incoming packets until the user exits the program.
/// Expects two env arguments: <program name, port>.
pub fn main() {
    let args = env::args();
    let port = extract_port_from_args(args);
    let address = format!("localhost:{}", port);

    let mut server = Server::new();
    server.listen(&address);
    loop_until_exit_requested(stdin().lock());
    server.stop_listening();
}

/// Returns a localhost address based on the port provided using the `-p` flag.
fn extract_port_from_args(mut args: Args) -> String {
    loop {
        match args.next() {
            Some(maybe_port_flag) => {
                if maybe_port_flag == "-p" {
                    break;
                } else {
                    continue;
                }
            },
            None => {
                let default_port = "10005";
                println!("No port provided. Using default of '{}'.", default_port);
                return default_port.to_string();
            }
        }
    }

    let passed_port = args.next().expect("Flag \"-p\" used but no port provided.");

    let port_is_numeric = passed_port.parse::<i32>().is_ok();
    assert!(port_is_numeric, "Flag \"-p\" used but port had incorrect format: {}.", passed_port);

    println!("Using provided port of {}.", passed_port);
    return passed_port;
}

/// Loop until the reader reads the word 'exit' (plus optional whitespace).
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