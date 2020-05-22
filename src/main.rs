use std::env;
use std::net::TcpListener;
use std::thread;
use std::io;

// TODO: Write two programs, have them communicate over sockets.
    // TODO: Parse the incoming packets.
fn main() {
    let args: Vec<String> = env::args().collect();
    let port = &args[1];
    let address = format!("localhost:{}", port);

    thread::spawn(move || {
        let listener = TcpListener::bind(address).expect("Failed to bind to address.");

        for stream in listener.incoming() {
            println!("{:?}", stream.expect("Connection failed."));
        }
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