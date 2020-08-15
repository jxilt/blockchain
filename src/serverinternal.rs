use std::io::{ErrorKind::WouldBlock};
use std::net::{TcpListener};
use std::sync::mpsc::{channel, Sender, Receiver};
use crate::handler::{Handler};
use std::thread::{spawn};
use std::io::{BufReader, BufWriter};

/// A TCP server.
pub struct ServerInternal {
    // Used to interrupt the listening thread.
    interrupt_sender: Option<Sender<u8>>
}

impl ServerInternal {
    pub fn new() -> ServerInternal {
        ServerInternal {
            interrupt_sender: None
        }
    }

    /// Sets up a channel between the main thread and the TCP listening thread. Starts listening 
    /// for TCP connections at the given address on a separate thread. Handles incoming 
    /// connections using the handler provided.
    pub fn listen <T: Handler + Send + 'static> (&mut self, address: &String, handler: T) {
        // TODO: Check if already listening. If so, abort.
        let interrupt_receiver = self.create_channel();
        ServerInternal::handle_incoming(address, interrupt_receiver, handler);
    }

    /// Stops listening for TCP connections.
    pub fn stop_listening(&mut self) {
        match &self.interrupt_sender {
            Some(sender) => {
                sender.send(0).expect("Failed to send interrupt request-handling thread.");
                self.interrupt_sender = None;
            },
            None => ()
        }
    }

    /// Sets up a channel between the main thread and the request-handling thread.
    fn create_channel(&mut self) -> Receiver<u8> {
        let (interrupt_sender, interrupt_receiver) = channel::<u8>();
        self.interrupt_sender = Some(interrupt_sender);
        return interrupt_receiver;
    }

    /// Starts listening for TCP connections at the given address on a separate thread. Handles 
    /// incoming connections using the handler provided.
    fn handle_incoming <T: Handler + Send + 'static> (address: &String, interrupt_receiver: Receiver<u8>, handler: T) {        
        let tcp_listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true).expect("Failed to set listener to non-blocking.");
        
        spawn(move || {
            for maybe_stream in tcp_listener.incoming() {
                match maybe_stream {
                    Ok(stream) => {
                        // We reverse the non-blocking behaviour set at the listener level.
                        stream.set_nonblocking(false).expect("Failed to set stream to blocking.");
                        
                        let reader = BufReader::new(&stream);
                        let writer = BufWriter::new(&stream);
                        handler.handle(reader, writer);
                    },
                    // The listener has not received a new connection yet.
                    Err(e) if e.kind() == WouldBlock => {
                        // We check for an interrupt.
                        if interrupt_receiver.try_recv().is_ok() {
                            break;
                        }
                        // TODO: Consider adding a sleep here.
                    },
                    // TODO: Handle error.
                    Err(_) => ()
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{BufReader, BufWriter, BufRead, Write};
    use std::sync::atomic::{AtomicU16, Ordering};
    use crate::handler::DummyHandler;
    use crate::serverinternal::ServerInternal;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn start_server(address: &String) -> ServerInternal {
        let mut server = ServerInternal::new();
        let handler = DummyHandler {};
        server.listen(address, handler);

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

        TcpStream::connect(address).unwrap();

        server.stop_listening();
    }

    #[test]
    fn server_can_be_stopped() {
        let address = get_address();
        let mut server = start_server(&address);

        server.stop_listening();

        TcpStream::connect(address).unwrap_err();
    }

    #[test]
    fn server_responds_to_packets() {
        let address = get_address();
        let mut server = start_server(&address);

        let stream = TcpStream::connect(address).expect("Failed to connect to server.");
        write_to_stream(&stream, b"\n");
        let response = get_response(&stream);

        assert_eq!(response, "DUMMY\n");

        server.stop_listening();
    }

    #[test]
    fn server_can_handle_concurrent_connections() {
        let address = get_address();
        let mut server = start_server(&address);

        // Interleaved connections - write to both, then read from both.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b"\n");
        write_to_stream(&second_stream, b"\n");
        let first_response = get_response(&first_stream);
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        // Nested connections - write to first, write then read from the second, then read from the first.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_stream(&first_stream, b"\n");
        write_to_stream(&second_stream, b"\n");
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("DUMMY\n", first_response);
        assert_eq!("DUMMY\n", second_response);

        server.stop_listening();
    }
}