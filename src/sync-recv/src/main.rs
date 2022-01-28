use std::net::{TcpListener, TcpStream, Shutdown};
use std::env;
use std::thread;
use std::io::{Read};

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; 4*1024]; // using 4KiB buffer
    while match stream.read(&mut data) {
        Ok(size) => {
            if size > 0 {
                println!("received data: size={}", size);
                true
            }
            else {
                false
            }
        },
        Err(_) => {
            println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("sync-recv <url>");
        return;
    }
    println!("url: {}", &args[1]);

    let url = &args[1];
    let listener = TcpListener::bind(url).unwrap();

    println!("Server listening on {}", url);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move|| {
                    // connection succeeded
                    handle_client(stream)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
                /* connection failed */
            }
        }
    }
    // close the socket server
    drop(listener);
}
