use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;
use std::str;
use std::io::{BufRead, Write, BufReader, BufWriter};

// TODO: Document functions.

// TODO: Add method to stop listening.
pub fn listen(address: String) {
    let listener = TcpListener::bind(address).expect("Failed to bind to address.");
    // The listener has its own thread, and generates a thread for each incoming connection.
    thread::spawn(move || {
        listener.incoming().for_each(|incoming| {
            thread::spawn(move || {
                handle_incoming(incoming);
            });
        });
    });
}

// TODO: Move away from just adding threads indefinitely.
// TODO: Store packets before ACKing.
fn handle_incoming(incoming: Result<TcpStream, io::Error>) {
    let stream = incoming.expect("Connection failed.");

    let buf_reader = BufReader::new(&stream);
    let contents = check_packet(buf_reader);

    let mut buf_writer = BufWriter::new(&stream);
    write_response(&mut buf_writer, contents);
}

fn check_packet<R: BufRead>(mut reader: R) -> Result<(), String> {
    let mut line = String::new();
    // TODO: Handle packets without any new-lines.
    reader.read_line(&mut line).expect("Reading failed.");

    let tokens = line.split_whitespace().collect::<Vec<&str>>();

    return match tokens[..] {
        ["BLOCKCHAIN", "1.0"] => Ok(()),
        _ => Err("Unrecognised packet.".to_string())
    }
}

fn write_response<W: Write>(writer: &mut W, contents: Result<(), String>) {
    // TODO: Pass error contents back to user.
    match contents {
        Ok(()) => writer.write(b"ACK\n").expect("Writing failed."),
        Err(_) => writer.write(b"ERR\n").expect("Writing failed.")
    };
}

#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{BufReader, BufWriter, BufRead, Write};

    fn write_to_listener_and_get_response(port: u16, packet_to_write: &[u8]) -> String {
        let address = format!("localhost:{}", port);
        super::listen(address.to_string());

        let client = TcpStream::connect(address).expect("Failed to connect to server.");
        let mut buf_reader = BufReader::new(&client);
        let mut buf_writer = BufWriter::new(&client);

        buf_writer.write(packet_to_write).expect("Failed to write packet.");
        buf_writer.flush().expect("Failed to flush buffer.");

        let mut response = String::new();
        buf_reader.read_line(&mut response).expect("Failed to read line.");
        return response;
    }

    #[test]
    fn listen_responds_err_to_invalid_packets() {
        let invalid_packets: Vec<&[u8]> = vec![
            b"\n", // Empty packet.
            b"BLOCKCHAIN\n", // First half of a valid packet.
            b"1.0\n" // Second half of a valid packet.
        ];

        let mut starting_port = 10005;

        for invalid_packet in invalid_packets {
            let response = write_to_listener_and_get_response(starting_port, invalid_packet);
            assert_eq!("ERR\n".to_string(), response);
            starting_port += 1;
        }
    }

    #[test]
    fn listen_responds_ack_to_valid_packets() {
        let valid_packet = b"BLOCKCHAIN 1.0\n";

        let response = write_to_listener_and_get_response(10005, valid_packet);
        assert_eq!("ACK\n".to_string(), response);
    }

    // TODO: Test multiple connections.
}