use std::io::{ErrorKind::WouldBlock};
use std::net::{TcpListener};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{JoinHandle, spawn};
use crate::handler::{Handler};

/// A TCP listener.
pub struct Listener<T: Handler + Send + 'static> {
    // Used to send interrupts to stop the listener.
    interrupt_sender: Option<Sender<u8>>,
    // Used to handle incoming connections.
    handler: T,
    // TODO: Document
    join_handle: Option<JoinHandle<()>>
}

impl <T: Handler + Send + 'static> Listener<T> {
    /// Starts listening for TCP connections at the given address on a separate thread. Handles incoming 
    /// connections using the handler provided.
    pub fn new(handler: T) -> Listener<T> {
        Listener {
            interrupt_sender: None,
            handler: handler,
            join_handle: None
        }
    }

    pub fn listen(self, address: String) {
        let (interrupt_sender, interrupt_receiver) = channel::<u8>();
        self.interrupt_sender = Some(interrupt_sender);

        // We create the listener outside the thread to be sure it is set up before we continue.
        let tcp_listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
        // We set the listener to non-blocking so that we can check for interrupts, below.
        tcp_listener.set_nonblocking(true).expect("Failed to set listener to non-blocking.");

        // The listener needs its own thread to listen for incoming connections.
        let listener_handle = spawn(move || Listener::handle_incoming(tcp_listener, interrupt_receiver, self.handler));

        self.join_handle = Some(listener_handle);
    }

    /// Stops listening for TCP connections. The listener cannot be restarted.
    pub fn stop_listening(self) {
        self.interrupt_sender.send(0).ok();
        self.listener_handle.join().ok();
    }

    /// Handles incoming TCP packets. Interrupts if an interrupt is received.
    fn handle_incoming(listener: TcpListener, interrupt_receiver: Receiver<u8>, handler: T) {
        for stream in listener.incoming() {
            // TODO: Handle multiple incoming streams concurrently.
            match stream {
                Ok(stream) => handler.handle(stream),
                // The listener has not received a new connection yet.
                Err(e) if e.kind() == WouldBlock => {
                    // We check for an interrupt.
                    if interrupt_receiver.try_recv().is_ok() {
                        break;
                    }
                    // TODO: Consider adding a sleep here.
                },
                Err(_) => ()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{BufReader, BufWriter, BufRead, Write};
    use std::sync::atomic::{AtomicU16, Ordering};
    use crate::handler::DummyHandler;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn write_to_listener(stream: &TcpStream, packet_to_write: &[u8]) {
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
        println!("{}", old_port);
        return format!("localhost:{}", old_port.to_string());
    }

    #[test]
    fn listener_allows_connections() {
        let address = get_address();
        let handler = DummyHandler {};
        let listener = crate::Listener::new(address.to_string(), handler);

        TcpStream::connect(address.to_string()).unwrap();

        listener.stop_listening();
    }

    #[test]
    fn listener_can_be_interrupted() {
        let address = get_address();
        let handler = DummyHandler {};
        let listener = crate::Listener::new(address.to_string(), handler);

        listener.stop_listening();

        TcpStream::connect(address.to_string()).unwrap_err();
    }

    #[test]
    fn listener_responds_to_packets() {
        let address = get_address();
        let handler = DummyHandler {};
        let listener = crate::Listener::new(address.to_string(), handler);

        let stream = TcpStream::connect(address).expect("Failed to connect to server.");
        write_to_listener(&stream, b"\n");
        let response = get_response(&stream);

        assert_eq!(response, "DUMMY\n");

        listener.stop_listening();
    }

    #[test]
    fn listener_can_handle_concurrent_connections() {
        let address = get_address();
        let handler = DummyHandler {};
        let listener = crate::Listener::new(address.to_string(), handler);

        // Interleaved connections - write to both, then read from both.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_listener(&first_stream, b"\n");
        write_to_listener(&second_stream, b"\n");
        let first_response = get_response(&first_stream);
        let second_response = get_response(&second_stream);

        assert_eq!("DUMMY\n".to_string(), first_response);
        assert_eq!("DUMMY\n".to_string(), second_response);

        // Nested connections - write to first, write then read from the second, then read from the first.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_listener(&first_stream, b"\n");
        write_to_listener(&second_stream, b"\n");
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("DUMMY\n".to_string(), first_response);
        assert_eq!("DUMMY\n".to_string(), second_response);

        listener.stop_listening();
    }
}