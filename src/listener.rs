use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;
use std::str;
use std::io::{BufRead, Write, BufReader, BufWriter};
use std::sync::mpsc;

/// Listens for TCP connections at the given address.
/// 
/// Expects packets of the form "BLOCKCHAIN 1.0\n", to which it will respond 
/// "ACK\n". For any other packet, it will respond "ERR\n".
pub fn listen(receiver: mpsc::Receiver<u8>, address: String) {
    let listener = TcpListener::bind(address).expect("Failed to bind listener to address.");
    listener.set_nonblocking(true).expect("Failed to set listener as non-blocking.");

    // The listener has its own thread, and generates a thread for each incoming connection.
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(_) => {
                    thread::spawn(move || {
                        handle_incoming(stream);
                    });
                },
                // The listener has not received a new connection yet.
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // TODO: Consider adding a sleep here.
                    // If we received a kill signal, we stop processing incoming connections.
                    match receiver.try_recv() {
                        Ok(_) => {
                            println!("Stopping processing incoming connections.");
                            break
                        },
                        // TODO: Better way to ignore error?
                        Err(_) => {}
                    }
                },
                Err(e) => {
                    // TODO: Print err message in panic.
                    println!("{}", e);
                    panic!(e)
                }
            }
        } 
    });
}

pub fn stop_listening(sender: mpsc::Sender<u8>) {
    sender.send(0).expect("The receiver has already hung up.");
}

// TODO: Move away from just adding threads indefinitely.
// TODO: Store packets before ACKing.
fn handle_incoming(incoming: Result<TcpStream, io::Error>) {
    let stream = incoming.expect("Incoming connection failed.");

    let buf_reader = BufReader::new(&stream);
    let contents = check_packet(buf_reader);

    let mut buf_writer = BufWriter::new(&stream);
    write_response(&mut buf_writer, contents);
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

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{BufReader, BufWriter, BufRead, Write};
    use std::sync::mpsc;

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

    #[test]
    fn listen_responds_err_to_invalid_packets() {
        let address = "localhost:10005";
        let (sender, receiver) = mpsc::channel::<u8>();
        super::listen(receiver, address.to_string());

        let invalid_packets: Vec<&[u8]> = vec![
            b"\n", // Empty packet.
            b"BLOCKCHAIN\n", // First half of a valid packet.
            b"1.0\n" // Second half of a valid packet.
        ];

        for invalid_packet in invalid_packets {
            let response = write_to_listener_and_get_response(address.to_string(), invalid_packet);
            assert_eq!("ERR\n".to_string(), response);
        }

        super::stop_listening(sender);
    }

    #[test]
    fn listen_responds_ack_to_valid_packets() {
        let address = "localhost:10005";
        let (sender, receiver) = mpsc::channel::<u8>();
        super::listen(receiver, address.to_string());

        let valid_packet = b"BLOCKCHAIN 1.0\n";

        let response = write_to_listener_and_get_response(address.to_string(), valid_packet);
        assert_eq!("ACK\n".to_string(), response);

        super::stop_listening(sender);
    }

    // #[test]
    fn listen_responds_to_multiple_connections_concurrently() {
        let address = "localhost:10005";
        let (sender, receiver) = mpsc::channel::<u8>();
        super::listen(receiver, address.to_string());

        let valid_packet = b"BLOCKCHAIN 1.0\n";
        let invalid_packet = b"\n";

        // To test concurrent connections, we nest a second write/read inside 
        // the first.
        let first_stream = TcpStream::connect(address).expect("Failed to connect to server.");
        let second_stream = TcpStream::connect(address).expect("Failed to connect to server.");
        write_to_listener(&first_stream, valid_packet);
        write_to_listener(&second_stream, invalid_packet);
        let second_response = get_response(&second_stream);
        let first_response = get_response(&first_stream);

        assert_eq!("ACK\n".to_string(), first_response);
        assert_eq!("ERR\n".to_string(), second_response);

        super::stop_listening(sender);
    }

    // TODO: Test of stopping listening.
}