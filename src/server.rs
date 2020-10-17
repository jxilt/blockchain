use std::io::{ErrorKind::WouldBlock};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

use crate::handler::{Handler, HttpHandler};
use crate::servererror::Result;
use std::collections::HashMap;

/// A TCP server.
pub struct Server { }

impl Server {
    /// Listens for and handles incoming TCP connections on the given address. Does not block the
    /// main thread.
    pub fn start(port: &str, db_connection_string: &str, routes: HashMap<String, String>) -> Result<ServerHandle> {
        let handler = HttpHandler::new(db_connection_string, routes)?;
        let server_handle = ServerInternal::start(port, handler)?;
        return Ok(server_handle);
    }
}

/// The class wrapped by `Server` that allows a custom handler to be injected for testing.
pub struct ServerInternal {

}

impl ServerInternal {
    /// Listens for and handles incoming TCP connections on the given port, using the handler
    /// provided. Does not block the main thread. Returns a handler for stopping the server.
    pub fn start<T: Handler + Sync + Send + 'static>(port: &str, handler: T) -> Result<ServerHandle> {
        // This channel is used to interrupt the TCP listening thread.
        let (interrupt_sender, interrupt_receiver)  = channel::<u8>();
        ServerInternal::listen::<T>(port, handler, interrupt_receiver)?;
        let server_handle = ServerHandle { interrupt_sender };
        return Ok(server_handle);
    }

    /// Listens for and handles incoming TCP connections on the given port, using the handler
    /// provided. Does not block the main thread. Stops listening if an interrupt is received.
    fn listen<T: Handler + Sync + Send + 'static>(port: &str, handler: T, interrupt_receiver: Receiver<u8>) -> Result<()> {
        let address = format!("0.0.0.0:{}", port);
        let tcp_listener = TcpListener::bind(address)?;

        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true)?;

        // We create a reference to the handler that can be shared across threads.
        let handler_arc = Arc::new(handler);

        // We listen on a separate thread.
        spawn(move || {
            for maybe_stream in tcp_listener.incoming() {
                match maybe_stream {
                    // We spin up a new thread to handle each incoming stream.
                    Ok(stream) => {
                        let handler_arc_clone = handler_arc.clone();
                        spawn(move || ServerInternal::handle_tcp_stream::<T>(stream, handler_arc_clone));
                    }
                    // The listener has not received a new connection yet.
                    Err(e) if e.kind() == WouldBlock => {
                        // We check for an interrupt.
                        if interrupt_receiver.try_recv().is_ok() {
                            break;
                        }
                    }
                    // We choose to panic, rather than passing the error back to the main thread.
                    Err(e) => panic!(e)
                }
            }
        });

        return Ok(());
    }

    /// Handles an incoming TCP connection, using the handler provided.
    fn handle_tcp_stream<T: Handler>(stream: TcpStream, handler: Arc<T>) -> Result<()> {
        // We reverse the non-blocking behaviour set at the listener level.
        stream.set_nonblocking(false)?;

        let reader = BufReader::new(&stream);
        let writer = BufWriter::new(&stream);
        return handler.handle(reader, writer);
    }
}

/// The handle returned when starting a TCP server, allowing the server to be brought to a halt.
pub struct ServerHandle {
    // Used to interrupt the TCP listening thread.
    interrupt_sender: Sender<u8>
}

impl ServerHandle {
    /// Brings the corresponding TCP server to a halt.
    pub fn stop_listening(&mut self) -> Result<()> {
        self.interrupt_sender.send(0)?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, BufWriter, Write};
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::handler::DummyHandler;
    use crate::server::{ServerInternal, ServerHandle};

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn get_port() -> String {
        return PORT.fetch_add(1, Ordering::Relaxed).to_string();
    }

    fn start_server(port: &str) -> ServerHandle {
        return ServerInternal::start(port, DummyHandler {}).unwrap();
    }

    fn write_to_stream(stream: &TcpStream, packet_to_write: &[u8]) {
        let mut buf_writer = BufWriter::new(stream);
        buf_writer.write(packet_to_write).unwrap();
        buf_writer.flush().unwrap();
    }

    fn get_response(stream: &TcpStream) -> String {
        let mut buf_reader = BufReader::new(stream);
        let mut response = String::new();
        buf_reader.read_line(&mut response).unwrap();
        return response;
    }

    #[test]
    fn server_can_be_stopped() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        server_handle.stop_listening().unwrap();

        let result = TcpStream::connect(address);
        assert!(result.is_err());
    }

    #[test]
    fn server_allows_connections() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        TcpStream::connect(address).unwrap();

        server_handle.stop_listening().unwrap();
    }

    #[test]
    fn server_responds_to_packets() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        let stream = TcpStream::connect(address).unwrap();
        write_to_stream(&stream, b" ");
        let response = get_response(&stream);

        assert_eq!(response, "DUMMY\n");

        server_handle.stop_listening().unwrap();
    }

    #[test]
    fn server_allows_multiple_connections_serially() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b" ");
        let first_response = get_response(&first_stream);

        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&second_stream, b" ");
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server_handle.stop_listening().unwrap();
    }

    #[test]
    fn server_allows_multiple_connections_concurrently() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        // Interleaved connections - write to both, then read from both.
        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b" ");
        write_to_stream(&second_stream, b" ");
        let first_response = get_response(&first_stream);
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        // Nested connections - write to first, write then read from the second, then read from the first.
        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b" ");
        write_to_stream(&second_stream, b" ");
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server_handle.stop_listening().unwrap();
    }

    #[test]
    fn server_handles_connections_in_parallel() {
        let port = get_port();
        let mut server_handle = start_server(&port);
        let address = format!("0.0.0.0:{}", port);

        // Creates an infinite loop on the first connection using the '#' special character.
        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b"#");

        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&second_stream, b" ");
        let response = get_response(&second_stream);

        // Still get a response on the second connection.
        assert_eq!("DUMMY\n", response);

        server_handle.stop_listening().unwrap();
    }
}