use std::io::{ErrorKind::WouldBlock};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

use crate::handler::{Handler};
use std::sync::Arc;

/// The TCP server itself.
pub struct ServerInternal <T: Handler + Sync + Send + 'static> {
    // Used to interrupt the TCP listening thread.
    interrupt_sender: Option<Sender<u8>>,
    // Uses to handle requests. An Arc is used to allow the handler to be shared across responder
    // threads.
    handler: Arc<T>
}

impl <T: Handler + Sync + Send + 'static> ServerInternal<T> {
    pub fn new(handler: T) -> ServerInternal<T> {
        ServerInternal {
            // An interrupt send is set when the server starts listening.
            interrupt_sender: None,
            handler: Arc::new(handler)
        }
    }

    /// Sets up an interrupt to kill the main server thread as needed. Then listens for and handles
    /// incoming TCP connections on the given address, using a separate thread.
    pub fn listen(&mut self, address: &String) -> Result<(), String> {
        return match self.create_interrupt_channel() {
            Err(e) => Err(e),
            Ok(interrupt_receiver) => {
                ServerInternal::listen_for_tcp_connections(self, address, interrupt_receiver);
                Ok(())
            }
        }
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) {
        // TODO: Err is returned when interrupting a closed server, with tests.
        match &self.interrupt_sender {
            None => (),
            Some(sender) => {
                sender.send(0).expect("Failed to interrupt the TCP listening thread.");
                self.interrupt_sender = None;
            }
        }
    }

    /// Creates a channel between the main thread and the TCP listening thread, in order to allow
    /// us to interrupt the latter.
    fn create_interrupt_channel(&mut self) -> Result<Receiver<u8>, String> {
        return match &self.interrupt_sender {
            Some(_sender) => Err("The server is already listening.".to_string()),
            None => {
                let (interrupt_sender, interrupt_receiver) = channel::<u8>();
                self.interrupt_sender = Some(interrupt_sender);
                Ok(interrupt_receiver)
            }
        }
    }

    /// Listens for and handles incoming TCP connections on the given address, using a separate
    /// thread.
    // TODO: Pass down handler, not self.
    // TODO: Return results from functions, here and more generally.
    fn listen_for_tcp_connections(&self, address: &String, interrupt_receiver: Receiver<u8>) {
        let tcp_listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true).expect("Failed to set listener to non-blocking.");

        let handler_arc = Arc::clone(&self.handler);
        spawn(move || {
            for maybe_stream in tcp_listener.incoming() {
                match maybe_stream {
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
                    },
                    Err(e) => panic!(e)
                }
            }
        });
    }

    fn handle_tcp_stream<U: Handler + Sync + Send + 'static>(stream: TcpStream, handler: Arc<U>) {
        // We reverse the non-blocking behaviour set at the listener level.
        stream.set_nonblocking(false).expect("Failed to set stream to blocking.");

        let reader = BufReader::new(&stream);
        let writer = BufWriter::new(&stream);
        handler.handle(reader, writer);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, BufWriter, Write};
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicU16, Ordering};

    use crate::handler::{DummyHandler};
    use crate::serverinternal::ServerInternal;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn start_server(address: &String) -> ServerInternal<DummyHandler> {
        let mut server = ServerInternal::new(DummyHandler {});
        server.listen(address).expect("Failed to start the server.");

        return server;
    }

    fn write_to_stream(stream: &TcpStream, packet_to_write: &[u8]) {
        let mut buf_writer = BufWriter::new(stream);
        buf_writer.write(packet_to_write).expect("Failed to write packet.");
        buf_writer.flush().expect("Failed to flush buffer.");
    }

    fn get_response(stream: &TcpStream) -> String {
        let mut buf_reader = BufReader::new(stream);
        let mut response = String::new();
        buf_reader.read_line(&mut response).expect("Failed to read line.");
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

        TcpStream::connect(address).expect("Failed to connect to server.");

        server.stop_listening();
    }

    #[test]
    fn server_can_be_stopped() {
        let address = get_address();
        let mut server = start_server(&address);

        server.stop_listening();

        let result = TcpStream::connect(address);
        assert!(result.is_err());
    }

    #[test]
    fn server_responds_to_packets() {
        let address = get_address();
        let mut server = start_server(&address);

        let stream = TcpStream::connect(address).expect("Failed to connect to server.");
        write_to_stream(&stream, b" ");
        let response = get_response(&stream);

        assert_eq!(response, "DUMMY\n");

        server.stop_listening();
    }

    #[test]
    fn server_can_only_listen_once_at_a_time() {
        let mut server = ServerInternal::new(DummyHandler {});
        let address = get_address();
        server.listen(&address).expect("Failed to start the server.");

        // Listening again on the same address should fail.
        let result = server.listen(&address);
        assert!(result.is_err());

        // Listening again on a different address should fail.
        let result = server.listen(&get_address());
        assert!(result.is_err());

        // Listening again after the server has been stopped should work.
        server.stop_listening();
        let result = server.listen(&address);
        assert!(result.is_ok());

        server.stop_listening();
    }

    #[test]
    fn there_can_be_multiple_connections_to_the_server_serially() {
        let address = get_address();
        let mut server = start_server(&address);

        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b" ");
        let first_response = get_response(&first_stream);

        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&second_stream, b" ");
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server.stop_listening();
    }

    #[test]
    fn there_can_be_multiple_connections_to_the_server_at_once() {
        let address = get_address();
        let mut server = start_server(&address);

        // Interleaved connections - write to both, then read from both.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b" ");
        write_to_stream(&second_stream, b" ");
        let first_response = get_response(&first_stream);
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        // Nested connections - write to first, write then read from the second, then read from the first.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b" ");
        write_to_stream(&second_stream, b" ");
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server.stop_listening();
    }

    #[test]
    fn the_server_handles_connections_in_parallel() {
        let address = get_address();
        let mut server = start_server(&address);

        // Creates an infinite loop on the first connection using the '#' special character.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b"#");

        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&second_stream, b" ");
        let response = get_response(&second_stream);

        // Still get a response on the second connection.
        assert_eq!("DUMMY\n", response);

        server.stop_listening();
    }
}