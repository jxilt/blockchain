use std::env;
use std::net::TcpListener;
use std::thread;
use std::io;

// TODO: Write two programs, have them communicate over sockets.
fn main() {
    let args: Vec<String> = env::args().collect();

    // TODO: Add a test to check default is used properly
    let port = match args.len() {
        0 | 1 => {
            let default_port = "10005";
            println!("No port provided. Using default of '{}'.", default_port);
            "10005"
        },
        _ => &args[1]
    };

    let address = format!("localhost:{}", port);

    // TODO: Acks on the incoming packets
    // TODO: Handle more than one incoming connection at once.
    

    let listener = TcpListener::bind(address).expect("Failed to bind to address.");


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