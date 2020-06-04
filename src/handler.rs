use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use crate::persistence::{InMemoryDbClient, DbClient};

/// A handler for TCP streams.
pub trait Handler {
    // Handles incoming connections.
    fn handle(&self, stream: TcpStream);
}

/// A dummy handler for testing.
pub struct DummyHandler {}

impl Handler for DummyHandler {
    /// Writes "DUMMY" to every stream.
    fn handle(&self, stream: TcpStream) {
        let mut writer = BufWriter::new(&stream);
        writer.write(b"DUMMY\n").expect("Writing failed.");
    }
}

/// A handler for flow sessions.
pub struct FlowSessionHandler {
    // Used to write flow session packets to the database.
    // TODO: Modify to take generic DB client.
    db_client: InMemoryDbClient
}

impl Handler for FlowSessionHandler {
    /// Checks the packet is properly formed, commits it to the database, and writes an ACK to the stream.
    fn handle(&self, stream: TcpStream) {
        // We reverse the non-blocking behaviour set at the listener level.
        stream.set_nonblocking(false).expect("Failed to set stream to blocking.");

        let reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let check_packet_result = FlowSessionHandler::check_packet(reader);
        // TODO: See what I can do about this ugly nesting.
        match check_packet_result {
            Ok(contents) => {
                let commit_result = self.db_client.commit(contents);

                match commit_result {
                    Ok(_) => writer.write(b"ACK\n").expect("Writing failed."),
                    Err(_) => writer.write(b"ERR\n").expect("Writing failed.")
                }
            },
            Err(_) => writer.write(b"ERR\n").expect("Writing failed.")
        };
    }
}

impl FlowSessionHandler {
    /// Checks the packet is properly formed.
    fn check_packet<R: BufRead>(mut reader: R) -> Result<String, String> {
        let mut line = String::new();
        // TODO: Handle packets not terminated by new-lines.
        reader.read_line(&mut line).expect("Reading from incoming connection failed.");

        let tokens = line.split_whitespace().collect::<Vec<&str>>();

        return match tokens[..] {
            // TODO: Store all lines, not just first.
            ["BLOCKCHAIN", "1.0"] => Ok(line.to_string()),
            _ => Err("Unrecognised packet.".to_string())
        }
    }
}

// TODO: Add tests.