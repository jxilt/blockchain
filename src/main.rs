use std::env;
use std::env::Args;
use std::io::{BufRead, stdin};

use crate::server::Server;
use crate::servererror::{Result, ServerError};

mod handler;
mod server;
mod servererror;
mod serverinternal;

const DEFAULT_PORT: &str = "10005";

/// Listens for incoming packets until the user exits the program.
/// Expects two env arguments: <program name, port>.
pub fn main() -> Result<()> {
    let args = env::args();
    let port = port_from_args_or_default(args)?;
    // TODO: Get database connection string from command line.

    let mut server = Server::new();
    server.set_port(port);
    // TODO: Update to a meaningful DB connection string.
    server.set_db_connection_string("www.google.com:80".into());
    server.register_route("/".into(), "./src/html/hello_world.html".into())?;
    server.start()?;

    loop_until_exit_requested(stdin().lock())?;
    server.stop()?;

    return Ok(());
}

/// Returns the port specified in the arguments using the '-p' flag, or returns the default port.
fn port_from_args_or_default(mut args: Args) -> Result<String> {
    loop {
        match args.next() {
            None => {
                println!("No port provided. Using default of '{}'.", DEFAULT_PORT);
                return Ok(DEFAULT_PORT.into());
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
        .ok_or(ServerError { message: "No argument passed after '-p' flag.".into() })?;

    port.parse::<i32>()?;
    println!("Using provided port of {}.", port);
    return Ok(port);
}

/// Loop until the reader reads the word 'exit' (plus optional whitespace).
fn loop_until_exit_requested<R: BufRead>(mut reader: R) -> Result<()> {
    let mut maybe_exit = String::new();

    loop {
        println!("Type 'exit' to exit.");
        maybe_exit.clear();

        reader.read_line(&mut maybe_exit)?;

        if maybe_exit.trim() == "exit" {
            return Ok(());
        }
    }
}