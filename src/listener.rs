use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io;
use std::str;
use std::io::{BufRead, Write, BufReader, BufWriter};

// TODO: Add a test that multiple connections can be handled.
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

// TODO: Add a test that bad connections fail.
// TODO: Move away from just adding threads indefinitely.
// TODO: Store packets before ACKing.
fn handle_incoming(incoming: Result<TcpStream, io::Error>) {
    let stream = incoming.expect("Connection failed.");

    let buf_read = BufReader::new(&stream);
    let contents = check_packet(buf_read);

    let mut buf_writer = BufWriter::new(&stream);
    write_response(&mut buf_writer, contents);
}

fn check_packet<R: BufRead>(mut reader: R) -> Result<(), String> {
    let mut line = String::new();
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
    #[test]
    fn check_packet_errors_on_invalid_packets() {
        let err = Err("Unrecognised packet.".to_string());

        let invalid_packets: Vec<&[u8]> = vec![
            b"", // Empty packet.
            b"\n", // Empty packet with new-line.
            b"BLOCKCHAIN", // First half of a valid packet.
            b"1.0" // Second half of a valid packet.
        ];
        let valid_packet: &[u8] = b"BLOCKCHAIN 1.0";

        for packet in invalid_packets {
            assert_eq!(err, super::check_packet(packet));
        }
        assert_eq!(Ok(()), super::check_packet(valid_packet));
    }

    #[test]
    fn write_response_writes_correct_response() {
        let valid_contents = Ok(());
        let invalid_contents = Err("".to_string());

        let mut valid_output = vec![];
        super::write_response(&mut valid_output, valid_contents);
        let valid_utf8 = String::from_utf8(valid_output).expect("Invalid UTF-8 string.");
        assert_eq!("ACK\n".to_string(), valid_utf8);

        let mut invalid_output = vec![];
        super::write_response(&mut invalid_output, invalid_contents);
        let invalid_utf8 = String::from_utf8(invalid_output).expect("Invalid UTF-8 string.");
        assert_eq!("ERR\n".to_string(), invalid_utf8);
    }

    // TODO: Listener tests - test can connect, test empty message handled, test protocol recognised, test non-protocol non-recognised, test multiple connections
}