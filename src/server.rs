use std::collections::HashMap;

use crate::handler::HttpHandler;
use crate::persistence::InMemoryDbClient;
use crate::servererror::ServerError;
use crate::serverinternal::ServerInternal;

/// A TCP server that wraps the ServerInternal class, to allow different request handlers to be
/// injected when testing the latter.
pub struct Server {
    // Work is delegated to the internal server.
    server_internal: ServerInternal<HttpHandler<InMemoryDbClient>>
}

impl Server {
    pub fn new(routes: HashMap<String, String>) -> Server {
        let handler_db_client = InMemoryDbClient::new();
        let handler = HttpHandler::new(handler_db_client, routes);

        Server {
            server_internal: ServerInternal::new(handler)
        }
    }

    /// Starts listening for TCP connections at the given address on a separate thread, and handles
    /// the incoming connections. A single server can only listen once at a time.
    pub fn listen(&mut self, address: &String) -> Result<(), ServerError> {
        return self.server_internal.listen(address);
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) -> Result<(), ServerError> {
        return self.server_internal.stop_listening();
    }
}