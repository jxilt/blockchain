use crate::serverinternal::{ServerInternal};
use crate::persistence::{InMemoryDbClient};
use crate::handler::{HttpHandler};
use std::collections::HashMap;

/// A TCP server.
pub struct Server {
    // Work is delegated to the internal server.
    server_internal: ServerInternal
}

/// Wraps the ServerInternal class, to allow different request handlers to be injected for testing.
impl Server {
    pub fn new() -> Server {
        let server_internal = ServerInternal::new();

        Server {
            server_internal
        }
    }

    /// Starts listening for TCP connections at the given address on a separate thread, and handles
    /// the incoming connections. A single server can only listen once at a time.
    pub fn listen(&mut self, address: &String) {
        // TODO: Allow a single handler to be shared across threads.
        // We create a fresh handler for each call to `listen`. This is because `listen` spawns a 
        // new thread that must own the handler.
        let db_client = InMemoryDbClient::new();
        let routes: HashMap<String, String> = [("/".to_string(), "./src/hello_world.html".to_string())]
            .iter().cloned().collect();
        let handler = HttpHandler::new(db_client, routes);

        self.server_internal.listen(address, handler).expect("Failed to start the server.");
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) {
        &self.server_internal.stop_listening();
    }
}