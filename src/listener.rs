use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;
use std::str;
use std::io::{BufRead, Write, BufReader, BufWriter};

pub fn listen(address: String) {
    // The listener has its own thread, and generates a thread for each incoming connection.
    thread::spawn(|| {
        let listener = TcpListener::bind(address).expect("Failed to bind to address.");
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
    // TODO: Handle packets without a newline.
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
    // TODO: Repurpose tests to only test public function `listen`.

    #[test]
    fn listen_responds_err_to_invalid_packets() {
        let address = "localhost:10005";
        super::listen(address.to_string());

        // TODO: Tests of packets with missing newline.
        let invalid_packets: Vec<&[u8]> = vec![
            b"\n", // Empty packet.
            b"BLOCKCHAIN\n", // First half of a valid packet.
            b"1.0\n" // Second half of a valid packet.
        ];

        for invalid_packet in invalid_packets {
            // TODO: Refactor common code in this and the following test.
            let client = TcpStream::connect(address).unwrap();
            let mut buf_reader = BufReader::new(&client);
            let mut buf_writer = BufWriter::new(&client);

            buf_writer.write(invalid_packet).unwrap();
            buf_writer.flush().unwrap();

            let mut response = String::new();
            buf_reader.read_line(&mut response).unwrap();

            assert_eq!("ERR\n".to_string(), response);
        }
    }

    #[test]
    fn listen_responds_ack_to_valid_packets() {
        let address = "localhost:10005";
        super::listen(address.to_string());

        let valid_packet: &[u8] = b"BLOCKCHAIN 1.0\n";
        let client = TcpStream::connect(address).unwrap();
        let mut buf_reader = BufReader::new(&client);
        let mut buf_writer = BufWriter::new(&client);

        buf_writer.write(valid_packet).unwrap();
        buf_writer.flush().unwrap();

        let mut response = String::new();
        buf_reader.read_line(&mut response).unwrap();
        assert_eq!("ACK\n".to_string(), response);
    }

    // TODO: Test multiple connections, test bad connections fail.
}