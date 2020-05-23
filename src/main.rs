use std::env;
use std::net::TcpListener;
use std::thread;
use std::io;

// TODO: Write two programs, have them communicate over sockets.
fn main() {
    let args: Vec<String> = env::args().collect();
    let port = allocate_port(&args);
    let address = format!("localhost:{}", port);

    let listener = TcpListener::bind(address).expect("Failed to bind to address.");

    // TODO: Add a test that multiple connections can be handled.
    // TODO: Match against the stream to handle errors, as shown in the docs.
    // TODO: Add a test that bad connections fail.
    // TODO: Work out if it's ok to just keep adding threads indefinitely.
    // TODO: Ack incoming packets
    thread::spawn(|| {
        listen(listener);
    });

    // TODO: Work out why the lock is required here.
    loop_until_exit(io::stdin().lock());
}

fn allocate_port(args: &[String]) -> &str {
    return match args.len() {
        0 | 1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            default_port
        },
        _ => &args[1]
    };
}

// We inject the reader and return the matched string to allow testing.
fn loop_until_exit<R: io::BufRead>(mut reader: R) -> String {
    loop {
        println!("Type 'exit' to exit.");
        let mut maybe_exit = String::new();
        reader.read_line(&mut maybe_exit).expect("Failed to read line.");
        if maybe_exit.trim() == "exit" {
            return maybe_exit.trim().to_string();
        }
    }
}

fn listen(listener: TcpListener) {
    listener.incoming()
        .for_each(|stream| {
            thread::spawn(move || {
                println!("{:?}", stream.expect("Connection failed."));
            });
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_port_is_allocated_if_less_than_two_args() {
        let default_port = "10005";
        let input_port = "10006";
        assert_ne!(input_port, default_port);

        let zero_args = vec![];
        let one_arg = vec!["program/being/run".to_string()];
        let two_args = vec!["program/being/run".to_string(), input_port.to_string()];

        // Default port is allocated if there are zero arguments.
        let allocated_port = crate::allocate_port(&zero_args);
        assert_eq!(default_port, allocated_port);

        // Default port is also allocated if there is one argument.
        let allocated_port = crate::allocate_port(&one_arg);
        assert_eq!(default_port, allocated_port);

        // Default port is not allocated if there are two arguments.
        let allocated_port = crate::allocate_port(&two_args);
        assert_eq!(input_port, allocated_port);
    }

    #[test]
    fn loop_exits_if_exit_is_typed() {
        // For these first two tests, the loop not running forever finishing indicates that the 'exit' line was picked up.
        let exit_line: &[u8] = b"exit\n";
        crate::loop_until_exit(exit_line);

        let exit_line_with_whitespace: &[u8] = b" exit \n";
        crate::loop_until_exit(exit_line_with_whitespace);

        // Checking that it's actually the 'exit' line that's picked up, rather than the two proceeding lines with similar words.
        let exit_line_with_other_similar_lines: &[u8] = b"zexit\nexitz\nexit\n";
        let exit_two = crate::loop_until_exit(exit_line_with_other_similar_lines);
        assert_eq!(exit_two, "exit");
    }
}