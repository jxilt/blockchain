use crate::serverinternal::{ServerInternal};
use crate::persistence::{InMemoryDbClient};
use crate::handler::{RequestHandler};

/// A TCP listener.
pub struct Server {
    server_internal: ServerInternal
}

impl Server {
    pub fn new() -> Server {
        let server_internal = ServerInternal::new();

        Server {
            server_internal
        }
    }

    /// Starts listening for TCP connections at the given address on a separate thread.
    /// Handles incoming connections.
    /// TODO: Document throws an exception if already listening.
    pub fn listen(&mut self, address: String) {
        let db_client = InMemoryDbClient::new();
        let handler = RequestHandler::new(db_client);
        self.server_internal.listen(address, handler);
    }

    /// Stops listening for TCP connections.
    /// TODO: Document throws an exception if not listening.
    pub fn stop_listening(&mut self) {
        &self.server_internal.stop_listening();
    }
}