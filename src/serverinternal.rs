use std::io::{ErrorKind::WouldBlock};
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;

use crate::handler::Handler;

/// The TCP server itself.
pub struct ServerInternal {
    // Used to interrupt the TCP listening thread.
    interrupt_sender: Option<Sender<u8>>
}

impl ServerInternal {
    pub fn new() -> ServerInternal {
        ServerInternal {
            interrupt_sender: None
        }
    }

    /// Listens for incoming TCP connections on the given address, and handles them using the
    /// handler provided. Does not block the main thread.
    pub fn listen <T: Handler + Send + Sync + 'static> (&mut self, address: &String, handler: T) -> Result<(), String> {
        return match &self.interrupt_sender {
            Some(_sender) => Err("The server is already listening.".to_string()),
            None => {
                let interrupt_receiver = self.create_interrupt_channel();
                ServerInternal::listen_for_tcp_connections(address, interrupt_receiver, handler);
                Ok(())
            }
        }
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) {
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
    fn create_interrupt_channel(&mut self) -> Receiver<u8> {
        let (interrupt_sender, interrupt_receiver) = channel::<u8>();
        self.interrupt_sender = Some(interrupt_sender);
        return interrupt_receiver;
    }

    /// Listens for incoming TCP connections on the given address, and handles them using the
    /// handler provided. Does not block the main thread, and uses a thread per connection.
    fn listen_for_tcp_connections <T: Handler + Send + Sync + 'static> (address: &String, interrupt_receiver: Receiver<u8>, handler: T) {
        let tcp_listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true).expect("Failed to set listener to non-blocking.");

        spawn(move || {
            let handler_arc = Arc::new(handler);

            for maybe_stream in tcp_listener.incoming() {
                match maybe_stream {
                    Ok(stream) => {
                        let handler_arc_clone= Arc::clone(&handler_arc);
                        spawn(move || ServerInternal::handle_tcp_stream(stream, handler_arc_clone));
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

    fn handle_tcp_stream <T: Handler + Send + 'static> (stream: TcpStream, handler: Arc<T>) {
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

    use crate::handler::DummyHandler;
    use crate::serverinternal::ServerInternal;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn start_server(address: &String) -> ServerInternal {
        let mut server = ServerInternal::new();
        let handler = DummyHandler {};
        server.listen(address, handler).expect("Failed to start the server.");

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
        let mut server = ServerInternal::new();
        let address = get_address();
        server.listen(&address, DummyHandler {}).expect("Failed to start the server.");

        // Listening again on the same address should fail.
        let result = server.listen(&address, DummyHandler {});
        assert!(result.is_err());

        // Listening again on a different address should fail.
        let result = server.listen(&get_address(), DummyHandler {});
        assert!(result.is_err());

        // Listening again after the server has been stopped should work.
        server.stop_listening();
        let result = server.listen(&address, DummyHandler {});
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