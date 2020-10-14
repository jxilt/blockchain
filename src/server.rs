use std::collections::HashMap;

use crate::handler::HttpHandler;
use crate::persistence::InMemoryDbClient;
use crate::servererror::{Result, ServerError};
use crate::serverinternal::ServerInternal;

/// The server. It allows its routes to be configured, before the server is started and eventually
/// stopped.
pub struct Server {
    // The routes registered so far.
    routes: HashMap<String, String>,
    // The actual work of listening for and handling requests is delegated to a ServerInternal
    // instance. This separation allows us to test the server separately from the process of route
    // registration, and independently of a specific Handler implementation.
    server_internal: Option<ServerInternal<HttpHandler<InMemoryDbClient>>>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            routes: HashMap::new(),
            server_internal: None,
        }
    }

    /// Registers a new route on the server. New routes are not picked up until the server is
    /// restarted.
    pub fn register(&mut self, path: String, file: String) {
        self.routes.insert(path, file);
    }

    /// Starts listening for and handling incoming TCP connections on the given address. Does not
    /// block the main thread. A given server can only listen once at a time.
    pub fn start(&mut self, address: &String) -> Result<()> {
        if self.server_internal.is_some() {
            return Err(ServerError { message: "Server is already listening.".to_string() });
        }

        // We set up the internal server to actually handle the requests.
        let handler_db_client = InMemoryDbClient::new();
        // We make a copy of the routes to provide to the handler.
        let routes = self.routes.clone();
        let handler = HttpHandler::new(handler_db_client, routes);
        self.server_internal = Some(ServerInternal::new(handler));

        return self.server_internal.as_mut()
            .ok_or(ServerError { message: "Server failed to start correctly.".to_string() })?
            .listen(address);
    }

    /// Stops listening for TCP connections.
    pub fn stop(&mut self) -> Result<()> {
        self.server_internal.as_mut()
            .ok_or(ServerError { message: "Server has not been started.".to_string() })?
            .stop_listening()?;

        // We reset the server so that it can be started again, with new registered routes.
        self.server_internal = None;

        return Ok(());
    }
}