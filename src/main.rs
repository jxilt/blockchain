use std::collections::HashMap;
use std::io::{BufRead, stdin};

use crate::server::Server;
use crate::servererror::Result;

mod handler;
mod server;
mod servererror;

// The port the server listens on.
const PORT: &str = "10005";
// The string the server uses to connect to its database.
// TODO: Update to meaningful DB connection string.
const DB_CONNECTION_STRING: &str = "www.google.com:80";

/// Starts a TCP server that listens for incoming packets until the user exits the program.
pub fn main() -> Result<()> {
    let routes = prepare_routes();
    let mut server_handle = Server::start(PORT, DB_CONNECTION_STRING, routes)?;

    loop_until_exit_requested(stdin().lock())?;
    server_handle.stop_listening()?;

    return Ok(());
}

/// Returns the routes that the server will serve.
fn prepare_routes() -> HashMap<String, String> {
    let mut routes = HashMap::new();
    routes.insert("/".into(), "./src/html/hello_world.html".into());
    return routes;
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