use std::env;
use std::io;
use std::io::BufRead;
use std::sync::mpsc;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

mod listener;

// TODO: Write two programs, have them communicate over sockets.
fn main() {
    let args = env::args().collect::<Vec<String>>();
    let address = address(&args);

    // TODO: Consider subbing this raw approach out for MQs.
    // TODO: Move the channel set-up back into some listener class.
    let (sender, receiver) = mpsc::channel::<u8>();
    let join_handle = listener::listen(receiver, address);

    // TODO: Work out why the lock is required here.
    loop_until_exit(io::stdin().lock());

    listener::stop_listening(sender, join_handle);
}

// Creates the address based on the port passed in on the command line.
// Two arguments are expected, with the port in second position.
fn address(args: &[String]) -> String {
    let port = match args.len() {
        0 => panic!("Too few arguments. Usage is '<program_name> <port>."),
        1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            default_port
        },
        2 => {
            let provided_port = &args[1];
            println!("Using provided port '{}'.", provided_port);
            provided_port
        },
        _ => panic!("Too many arguments. Usage is '<program_name> <port>.")
    };

    return format!("localhost:{}", port);
}

// Loop until the user types 'exit'.
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
    use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

    #[test]
    #[should_panic(expected = "Too few arguments. Usage is '<program_name> <port>.")]
    fn process_args_panics_with_zero_args() {
        let no_args = vec![];
        crate::address(&no_args);
    }

    #[test]
    #[should_panic(expected = "Too many arguments. Usage is '<program_name> <port>.")]
    fn process_args_panics_with_more_than_two_args() {
        let three_args = vec!["place".to_string(), "holder".to_string(), "values".to_string()];
        crate::address(&three_args);
    }

    #[test]
    fn process_args_allocates_a_default_port_if_necessary() {
        let default_port = 10005;
        let input_port = 10006;
        assert_ne!(input_port, default_port);
        let default_address = format!("localhost:{}", default_port);
        let address_with_input = format!("localhost:{}", input_port);

        let args_no_port_provided = vec!["program/being/run".to_string()];
        let args_port_provided = vec!["program/being/run".to_string(), input_port.to_string()];

        // Default port is allocated if there is one argument.
        let address_one = crate::address(&args_no_port_provided);
        assert_eq!(default_address, address_one);

        // Default port is not allocated if there are two arguments.
        let address_two = crate::address(&args_port_provided);
        assert_eq!(address_with_input, address_two);
    }

    #[test]
    fn loop_until_exit_exits_if_exit_is_typed() {
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