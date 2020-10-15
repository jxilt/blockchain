use std::collections::HashMap;

use crate::handler::HttpHandler;
use crate::servererror::{Result, ServerError};
use crate::serverinternal::ServerInternal;
use crate::server::ServerState::{Unstarted, Stopped, Started};

/// A webserver that listens for and responds to HTTP connections, at the port provided and using
/// the routes provided. The listening occurs on a separate thread.
pub struct Server {
    // The routes registered so far.
    routes: HashMap<String, String>,
    // The port the server listens on.
    port: Option<String>,
    // The database connection string the server uses for persistence.
    db_connection_string: Option<String>,
    // Listening for and handling requests is delegated to a ServerInternal instance. This instance
    // is only initialised once `start` is called, after the routes have been registered.
    server_internal: Option<ServerInternal<HttpHandler>>,
    // The current state the server is in. Used to prevent invalid transitions.
    server_state: ServerState
}

impl Server {
    pub fn new() -> Server {
        Server {
            routes: HashMap::new(),
            port: None,
            db_connection_string: None,
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

    /// Sets the port the server listens on.
    pub fn set_port(&mut self, port: String) {
        // TODO: Catch setting port after server is started.
        self.port = Some(port);
    }

    /// Sets the database connection string used by the server for persistence.
    pub fn set_db_connection_string(&mut self, db_connection_string: String) {
        // TODO: Catch setting DB connection string after server is started.
        self.db_connection_string = Some(db_connection_string);
    }

    /// Starts listening for and responding to HTTP connections, at the port provided and using
    /// the routes provided. The listening occurs on a separate thread. A webserver can only be
    /// started once.
    pub fn start(&mut self) -> Result<()> {
        match self.server_state {
            Unstarted => {
                self.server_state = Started;

                let db_connection_string = &self.db_connection_string.as_ref()
                    .ok_or(ServerError { message: "Database connection string must be set before starting the server".into() })?;
                let routes = self.routes.clone();
                let request_handler = HttpHandler::new(&db_connection_string, routes)?;
                self.server_internal = Some(ServerInternal::new(request_handler));

                let port = self.port.as_ref()
                    .ok_or(ServerError { message: "Database connection string must be set before starting the server".into() })?;
                let address = &format!("0.0.0.0:{}", port);

                return self.server_internal.as_mut().ok_or(
                    ServerError { message: "The server has had an internal issue.".into() }
                )?.listen(address);
            }
            _ => Err(ServerError { message: "The server can only be started while it is unstarted.".into() })
        }
    }

    /// Stops listening for HTTP connections. Once a server has been stopped, it cannot be
    /// restarted.
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

/// The states the server can be in.
enum ServerState {
    Unstarted,
    Started,
    Stopped
}

// TODO: Integration tests of everything.