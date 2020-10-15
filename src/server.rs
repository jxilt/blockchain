use std::collections::HashMap;

use crate::handler::HttpHandler;
use crate::servererror::{Result, ServerError};
use crate::serverinternal::ServerInternal;
use crate::server::ServerState::{Unstarted, Stopped, Started};

/// The server. It allows its routes to be configured, before the server is started and eventually
/// stopped.
pub struct Server {
    // The routes registered so far.
    routes: HashMap<String, String>,
    // Listening for and handling requests is delegated to a ServerInternal instance. This instance
    // is only initialised once `start` is called, after the routes have been registered.
    server_internal: Option<ServerInternal<HttpHandler>>,
    server_state: ServerState
}

impl Server {
    pub fn new() -> Server {
        Server {
            routes: HashMap::new(),
            server_internal: None,
            server_state: Unstarted
        }
    }

    /// Registers a new route on the server. New routes can only be installed before the server has
    /// been started.
    pub fn register_route(&mut self, path: String, file: String) -> Result<()> {
        return match self.server_state {
            Unstarted => {
                self.routes.insert(path, file);
                Ok(())
            },
            _  => Err( ServerError { message: "Routes can only be registered before the server has been started.".into() })
        };
    }

    /// Starts listening for and handling incoming HTTP connections on the given address. Does not
    /// block the main thread. A given server can only listen once at a time.
    pub fn start(&mut self, db_connection_string: &str, address: &str) -> Result<()> {
        match self.server_state {
            Unstarted => {
                self.server_state = Started;

                // TODO: Provide a flag to set to test mode, so a dummy handler can be injected and the
                //  server can be tested.
                let request_handler = HttpHandler::new(
                    db_connection_string,
                    // We provide a copy of the routes at the point in time the server is started.
                    self.routes.clone()
                )?;
                self.server_internal = Some(ServerInternal::new(request_handler));

                return self.server_internal.as_mut()
                    .ok_or(ServerError { message: "The server has had an internal issue.".into() })?
                    .listen(address);
            }
            _ => Err(ServerError { message: "The server can only be started while it is unstarted.".into() })
        }
    }

    /// Stops listening for TCP connections.
    pub fn stop(&mut self) -> Result<()> {
        return match self.server_state {
            Started => {
                self.server_state = Stopped;

                return self.server_internal.as_mut()
                    .ok_or(ServerError { message: "The server has had an internal issue.".into() })?
                    .stop_listening();
            },
            _ => Err(ServerError { message: "The server can only be stopped once it has been started.".into() })
        }
    }
}

/// The states a server can be in.
enum ServerState {
    Unstarted,
    Started,
    Stopped
}