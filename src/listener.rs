use std::io::{BufRead, Write, BufReader, BufWriter, ErrorKind::WouldBlock};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;

pub struct Listener {
    sender: Sender<u8>,
    join_handle: JoinHandle<()>
}

impl Listener {
    /// Listens for and responds to TCP connections at the given address.
    /// 
    /// Expects packets of the form "BLOCKCHAIN 1.0\n", to which it will respond 
    /// "ACK\n". For any other packet, it will respond "ERR\n".
    pub fn new(address: String) -> Listener {
        let (sender, receiver) = channel::<u8>();
        let join_handle = Listener::listen(receiver, address);

        Listener {
            sender,
            join_handle
        }
    }

    // Stop listening for TCP connections.
    pub fn stop_listening(self) {
        self.sender.send(0).ok();
        self.join_handle.join().ok();
    }

    fn listen(receiver: Receiver<u8>, address: String) -> JoinHandle<()> {
        let listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
        // We set the listener to non-blocking so that we can check for interrupts, below.
        listener.set_nonblocking(true).expect("Failed to set listener to non-blocking.");

        // The listener needs its own thread to listen for incoming connections.
        return thread::spawn(move || {
            // Each incoming connection gets its own thread to allow concurrent connections.
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        thread::spawn(move || {
                            Listener::handle_incoming(stream);
                        });
                    },
                    // The listener has not received a new connection yet.
                    Err(e) if e.kind() == WouldBlock => {
                        // We check for an interrupt.
                        if receiver.try_recv().is_ok() {
                            break;
                        }
                        // TODO: Consider adding a sleep here.
                    },
                    Err(_) => ()
                }
            }
        });
    }

    // TODO: Store packets before ACKing.
    fn handle_incoming(stream: TcpStream) {
        // We reverse the non-blocking behaviour set at the listener level.
        stream.set_nonblocking(false).expect("Failed to set stream to blocking.");

        let buf_reader = BufReader::new(&stream);
        let contents = Listener::check_packet(buf_reader);

        let mut buf_writer = BufWriter::new(&stream);
        Listener::write_response(&mut buf_writer, contents);
    }

    fn check_packet<R: BufRead>(mut reader: R) -> Result<(), String> {
        let mut line = String::new();
        // TODO: Handle packets not terminated by new-lines.
        reader.read_line(&mut line).expect("Reading from incoming connection failed.");

        let tokens = line.split_whitespace().collect::<Vec<&str>>();

        return match tokens[..] {
            ["BLOCKCHAIN", "1.0"] => Ok(()),
            _ => Err("Unrecognised packet.".to_string())
        }
    }

    fn write_response<W: Write>(writer: &mut W, contents: Result<(), String>) {
        match contents {
            Ok(()) => writer.write(b"ACK\n").expect("Writing failed."),
            Err(_) => writer.write(b"ERR\n").expect("Writing failed.")
        };
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{BufReader, BufWriter, BufRead, Write};
    use std::sync::atomic::AtomicU16;
    use std::sync::atomic::Ordering;

    // Used to allocate different ports for the listeners across tests.
    static PORT: AtomicU16 = AtomicU16::new(10000);

    fn write_to_listener_and_get_response(address: String, packet_to_write: &[u8]) -> String {
        let stream = TcpStream::connect(address).expect("Failed to connect to server.");
        write_to_listener(&stream, packet_to_write);
        return get_response(&stream);
    }

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
        return format!("localhost:{}", old_port.to_string());
    }

    #[test]
    fn listen_can_be_interrupted() {
        let address = get_address();
        let listener = super::Listener::new(address.to_string());

        listener.stop_listening();

        TcpStream::connect(address.to_string()).unwrap_err();
    }

    #[test]
    fn listen_responds_err_to_invalid_packets() {
        let address = get_address();
        super::Listener::new(address.to_string());

        let invalid_packets: Vec<&[u8]> = vec![
            b"\n", // Empty packet.
            b"BLOCKCHAIN\n", // First half of a valid packet.
            b"1.0\n" // Second half of a valid packet.
        ];

        for invalid_packet in invalid_packets {
            let response = write_to_listener_and_get_response(address.to_string(), invalid_packet);
            assert_eq!("ERR\n".to_string(), response);
        }
    }

    #[test]
    fn listen_responds_ack_to_valid_packets() {
        let address = get_address();
        super::Listener::new(address.to_string());

        let valid_packet = b"BLOCKCHAIN 1.0\n";

        let response = write_to_listener_and_get_response(address.to_string(), valid_packet);
        assert_eq!("ACK\n".to_string(), response);
    }

    #[test]
    fn listen_responds_to_multiple_connections_concurrently() {
        let address = get_address();
        super::Listener::new(address.to_string());

        let valid_packet = b"BLOCKCHAIN 1.0\n";
        let invalid_packet = b"\n";

        // Interleaved connections - write to both, then read from both.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_listener(&first_stream, valid_packet);
        write_to_listener(&second_stream, invalid_packet);
        let first_response = get_response(&first_stream);
        let second_response = get_response(&second_stream);

        assert_eq!("ACK\n".to_string(), first_response);
        assert_eq!("ERR\n".to_string(), second_response);

        // Nested connections - write to first, write then read from the second, then read from the first.
        let first_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address.to_string()).expect("Failed to connect to server.");
        write_to_listener(&first_stream, valid_packet);
        write_to_listener(&second_stream, invalid_packet);
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("ACK\n".to_string(), first_response);
        assert_eq!("ERR\n".to_string(), second_response);
    }
}