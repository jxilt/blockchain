use std::env;
use std::net::TcpListener;
use std::thread;
use std::io;

// TODO: Write two programs, have them communicate over sockets.
fn main() {
    let args: Vec<String> = env::args().collect();
    let port = allocate_port(args);
    let address = format!("localhost:{}", port);

    // TODO: Acks on the incoming packets

    let listener = TcpListener::bind(address).expect("Failed to bind to address.");

    // TODO: Create an extra thread to handle the for_each loop, or will be stuck forever.
    // TODO: Add a test that multiple threads can be handled.
    // TODO: Match against the stream to handle errors, as shown in the docs.
    // TODO: Add a test that bad connections fail.
    // TODO: Work out if it's ok to just keep adding tests indefinitely.
    listener.incoming()
        .for_each(|stream| {
            thread::spawn(move || {
                println!("{:?}", stream.expect("Connection failed."));
            });
    });

    loop {
        println!("Type 'exit' to exit.");
        let mut maybe_exit = String::new();
        io::stdin().read_line(&mut maybe_exit).expect("Failed to read line.");
        if maybe_exit.trim() == "exit" {
            break;
        }
    }
}

fn allocate_port(args: Vec<String>) -> String {
    return match args.len() {
        0 | 1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            default_port.to_string()
        },
        _ => args[1].to_string()
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_port_is_allocated_if_less_than_two_args() {
        let default_port = "10005";

        // Default port is allocated if there are zero arguments.
        let args = vec![];
        let allocated_port = crate::allocate_port(args);
        assert_eq!(default_port, allocated_port);

        // Default port is also allocated if there is one argument.
        let args = vec!["program/being/run".to_string()];
        let allocated_port = crate::allocate_port(args);
        assert_eq!(default_port, allocated_port);

        // Default port is not allocated if there are two arguments.
        let input_port = "10006";
        assert_ne!(input_port, default_port);
        let args = vec!["program/being/run".to_string(), input_port.to_string()];
        let allocated_port = crate::allocate_port(args);
        assert_eq!(input_port, allocated_port);
    }
}