use std::env;
use std::net::TcpListener;
use std::thread;
use std::io;
use std::str;
use std::io::{BufRead, Write, BufReader, BufWriter};

// TODO: Write two programs, have them communicate over sockets.
fn main() {
    let args = env::args().collect::<Vec<String>>();
    // TODO: Test that an exception is thrown if args are wrong.
    // TODO: Wrap the two statements below into a single function.
    let port = match process_args(&args) {
        Ok(port) => port,
        Err(e) => panic!(e)
    };
    let address = format!("localhost:{}", port);

    // TODO: Consider subbing this raw approach out for MQs.
    let listener = TcpListener::bind(address).expect("Failed to bind to address.");
    thread::spawn(|| {
        listen(listener);
    });

    // TODO: Work out why the lock is required here.
    loop_until_exit(io::stdin().lock());
}

// Processes the arguments and returns the port number.
// Two arguments are expected, with the port in second position.
fn process_args(args: &[String]) -> Result<String, String> {
    return match args.len() {
        0 => Err("Too few arguments. Usage is '<program_name> <port>.".to_string()),
        1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            Ok(default_port.to_string())
        },
        2 => {
            let provided_port = &args[1];
            println!("Using provided port '{}'.", provided_port);
            Ok(provided_port.to_string())
        },
        _ => Err("Too many arguments. Usage is '<program_name> <port>.".to_string())
    };
}

// TODO: Add a test that multiple connections can be handled.
// TODO: Add a test that bad connections fail.
// TODO: Move away from just adding threads indefinitely.
// TODO: Store packets before ACKing.
fn listen(listener: TcpListener) {
    listener.incoming()
        .for_each(|incoming| {
            thread::spawn(move || {
                let stream = incoming.expect("Connection failed.");

                let buf_read = BufReader::new(&stream);
                let contents = check_packet(buf_read);

                let mut buf_writer = BufWriter::new(&stream);
                write_response(&mut buf_writer, contents);
            });
    });
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

// We inject the reader and return the matched string to allow testing.
fn loop_until_exit<R: BufRead>(mut reader: R) -> String {
    loop {
        println!("Type 'exit' to exit.");
        let mut maybe_exit = String::new();
        reader.read_line(&mut maybe_exit).expect("Failed to read line.");
        if maybe_exit.trim() == "exit" {
            return maybe_exit.trim().to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn two_arguments_are_expected() {
        // TODO: Write tests of zero and 2+ args
    }

    #[test]
    fn default_port_is_allocated_if_less_than_two_args() {
        let default_port = "10005";
        let input_port = "10006";
        assert_ne!(input_port, default_port);

        let no_port_provided = vec!["program/being/run".to_string()];
        let port_provided = vec!["program/being/run".to_string(), input_port.to_string()];

        // Default port is allocated if there is one argument.
        let port = crate::process_args(&no_port_provided);
        assert!(port.is_ok() && port.unwrap() == default_port);

        // Default port is not allocated if there are two arguments.
        let port = crate::process_args(&port_provided);
        assert!(port.is_ok() && port.unwrap() == input_port);
    }

    #[test]
    fn check_packet_matches_packet() {
        let err = Err("Unrecognised packet.".to_string());

        let empty_packet: &[u8] = b"";
        let empty_packet_with_newline: &[u8] = b"";
        let first_half_packet: &[u8] = b"BLOCKCHAIN";
        let second_half_packet: &[u8] = b"1.0";
        
        let valid_packet: &[u8] = b"BLOCKCHAIN 1.0";

        assert_eq!(err, crate::check_packet(empty_packet));
        assert_eq!(err, crate::check_packet(empty_packet_with_newline));
        assert_eq!(err, crate::check_packet(first_half_packet));
        assert_eq!(err, crate::check_packet(second_half_packet));

        assert_eq!(Ok(()), crate::check_packet(valid_packet));
    }

    #[test]
    fn correct_response_is_written() {
        let valid_contents = Ok(());
        let invalid_contents = Err("".to_string());

        let mut valid_output = vec![];
        crate::write_response(&mut valid_output, valid_contents);
        let valid_utf8 = String::from_utf8(valid_output).expect("Invalid UTF-8 string.");
        assert_eq!("ACK\n".to_string(), valid_utf8);

        let mut invalid_output = vec![];
        crate::write_response(&mut invalid_output, invalid_contents);
        let invalid_utf8 = String::from_utf8(invalid_output).expect("Invalid UTF-8 string.");
        assert_eq!("ERR\n".to_string(), invalid_utf8);
    }

    // TODO: Listener tests - test can connect, test empty message handled, test protocol recognised, test non-protocol non-recognised, test multiple connections

    #[test]
    fn loop_exits_if_exit_is_typed() {
        let exit_line: &[u8] = b"exit\n";
        let exit_line_with_whitespace: &[u8] = b" exit \n";
        let exit_line_with_other_similar_lines: &[u8] = b"zexit\nexitz\nexit\n";

        // For these first two tests, the loop not running forever finishing indicates that the 'exit' line was picked up.
        crate::loop_until_exit(exit_line);
        crate::loop_until_exit(exit_line_with_whitespace);

        // Checking that it's actually the 'exit' line that's picked up, rather than the two proceeding lines with similar words.
        let exit_two = crate::loop_until_exit(exit_line_with_other_similar_lines);
        assert_eq!(exit_two, "exit");
    }
}