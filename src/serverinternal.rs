use std::io::{ErrorKind::WouldBlock};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

use crate::handler::Handler;
use crate::servererror::{Result, ServerError};

/// The TCP server itself.
pub struct ServerInternal<H: Handler> {
    // Used to interrupt the TCP listening thread.
    interrupt_sender: Option<Sender<u8>>,
    // Uses to handle requests. An Arc is used to allow the handler to be shared across responder
    // threads.
    handler: Arc<H>,
}

impl<T: Handler + Sync + Send + 'static> ServerInternal<T> {
    pub fn new(handler: T) -> ServerInternal<T> {
        ServerInternal {
            // This field is set when the server starts listening, and unset when it stops.
            interrupt_sender: None,
            handler: Arc::new(handler),
        }
    }

    /// Sets up an interrupt to kill the main server thread as needed. Then listens for and handles
    /// incoming TCP connections on the given address, using a separate thread. A given server can
    /// only listen once at a time.
    pub fn listen(&mut self, address: &str) -> Result<()> {
        let interrupt_receiver = self.create_interrupt_channel()?;
        return ServerInternal::listen_for_tcp_connections(address, interrupt_receiver, &self.handler);
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) -> Result<()> {
        let interrupt_sender = self.interrupt_sender.as_ref()
            .ok_or(ServerError { message: "No channel exists to interrupt listening thread.".to_string() })?;
        interrupt_sender.send(0)?;
        self.interrupt_sender = None;
        return Ok(());
    }

    /// Creates a channel between the main thread and the TCP listening thread, in order to allow
    /// us to interrupt the latter.
    fn create_interrupt_channel(&mut self) -> Result<Receiver<u8>> {
        if self.interrupt_sender.is_some() {
           return Err(ServerError { message: "Server is already listening.".to_string() });
        }

        let (interrupt_sender, interrupt_receiver) = channel::<u8>();
        self.interrupt_sender = Some(interrupt_sender);
        return Ok(interrupt_receiver);
    }

    /// Listens for and handles incoming TCP connections on the given address, using a separate
    /// thread.
    fn listen_for_tcp_connections(address: &str, interrupt_receiver: Receiver<u8>, handler: &Arc<T>) -> Result<()> {
        let tcp_listener = TcpListener::bind(address)?;
        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true)?;

        // We clone the Arc once here to avoid capturing a reference to self in the thread we spawn.
        let handler_arc = Arc::clone(handler);
        spawn(move || {
            for maybe_stream in tcp_listener.incoming() {
                match maybe_stream {
                    // We spin up a handler thread for the new connection.
                    Ok(stream) => {
                        let handler_arc_clone = handler_arc.clone();
                        spawn(move || ServerInternal::<T>::handle_tcp_stream(stream, handler_arc_clone));
                    }
                    // The listener has not received a new connection yet.
                    Err(e) if e.kind() == WouldBlock => {
                        // We check for an interrupt.
                        if interrupt_receiver.try_recv().is_ok() {
                            break;
                        }
                    }
                    // We choose to panic, instead of passing results back to the main thread.
                    Err(e) => panic!(e)
                }
            }
        });

        return Ok(());
    }

    fn handle_tcp_stream<H: Handler>(stream: TcpStream, handler: Arc<H>) -> Result<()> {
        // We reverse the non-blocking behaviour set at the listener level.
        stream.set_nonblocking(false)?;

        let reader = BufReader::new(&stream);
        let writer = BufWriter::new(&stream);

        return handler.handle(reader, writer);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, BufWriter, Write};
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::handler::DummyHandler;
    use crate::serverinternal::ServerInternal;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn start_server(address: &String) -> ServerInternal<DummyHandler> {
        let mut server = ServerInternal::new(DummyHandler {});
        server.listen(address).unwrap();
        return server;
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

    fn get_address() -> String {
        let old_port = PORT.fetch_add(1, Ordering::Relaxed);
        return format!("localhost:{}", old_port);
    }

    #[test]
    fn server_allows_connections() {
        let address = get_address();
        let mut server = start_server(&address);

        TcpStream::connect(address).unwrap();

        server.stop_listening().unwrap();
    }

    #[test]
    fn server_can_be_stopped() {
        let address = get_address();
        let mut server = start_server(&address);

        server.stop_listening().unwrap();

        let result = TcpStream::connect(address);
        assert!(result.is_err());
    }

    #[test]
    fn server_can_only_be_stopped_while_listening() {
        // Server cannot be stopped before starting listening initially.
        let mut server = ServerInternal::new(DummyHandler {});
        let result = server.stop_listening();
        assert!(result.is_err());

        // Server cannot be stopped when no longer listening.
        let address = get_address();
        server.listen(&address).unwrap();
        server.stop_listening().unwrap();
        let result = server.stop_listening();
        assert!(result.is_err());
    }

    #[test]
    fn server_responds_to_packets() {
        let address = get_address();
        let mut server = start_server(&address);

        let stream = TcpStream::connect(address).unwrap();
        write_to_stream(&stream, b" ");
        let response = get_response(&stream);

        assert_eq!(response, "DUMMY\n");

        server.stop_listening().unwrap();
    }

    #[test]
    fn server_can_only_listen_once_at_a_time() {
        let mut server = ServerInternal::new(DummyHandler {});
        let address = get_address();
        server.listen(&address).unwrap();

        // Listening again on the same address should fail.
        let result = server.listen(&address);
        assert!(result.is_err());

        // Listening again on a different address should fail.
        let result = server.listen(&get_address());
        assert!(result.is_err());

        // Listening again after the server has been stopped should work.
        server.stop_listening().unwrap();
        let result = server.listen(&address);
        assert!(result.is_ok());

        server.stop_listening().unwrap();
    }

    #[test]
    fn there_can_be_multiple_connections_to_the_server_serially() {
        let address = get_address();
        let mut server = start_server(&address);

        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b" ");
        let first_response = get_response(&first_stream);

        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&second_stream, b" ");
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server.stop_listening().unwrap();
    }

    #[test]
    fn there_can_be_multiple_connections_to_the_server_at_once() {
        let address = get_address();
        let mut server = start_server(&address);

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

        server.stop_listening().unwrap();
    }

    #[test]
    fn the_server_handles_connections_in_parallel() {
        let address = get_address();
        let mut server = start_server(&address);

        // Creates an infinite loop on the first connection using the '#' special character.
        let first_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&first_stream, b"#");

        let second_stream = TcpStream::connect(address.to_string()).unwrap();
        write_to_stream(&second_stream, b" ");
        let response = get_response(&second_stream);

        // Still get a response on the second connection.
        assert_eq!("DUMMY\n", response);

        server.stop_listening().unwrap();
    }
}