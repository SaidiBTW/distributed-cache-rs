use std::{
    collections::HashMap,
    io::Read,
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
};

type Cache = Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>;

fn handle_client(mut stream: TcpStream, cache: Cache) {
    let mut buffer = [0; 512];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let cache: Cache = Arc::new(RwLock::new(HashMap::new()));

    println!("Cache server listener on port 7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let cache_ref = Arc::clone(&cache);
                thread::spawn(move || {
                    handle_client(stream, cache_ref);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}
