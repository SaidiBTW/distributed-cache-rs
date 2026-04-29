use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

type Cache = Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>;

const MAX_KEY_SIZE: u32 = 1024; // 1 KB
const MAX_VALUE_SIZE: u32 = 1024 * 1024; //1MB

#[repr(u8)]
enum Command {
    Get = 1,
    Set = 2,
}

impl TryFrom<u8> for Command {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        match value {
            1 => Ok(Command::Get),
            2 => Ok(Command::Set),
            other => Err(other),
        }
    }
}

fn handle_client(mut stream: TcpStream, cache: Cache) {
    loop {
        //Set a timeout to prevent blocking forever
        if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
            eprintln!("Failed to set read timeout");
            break;
        }

        //Using 1 byte for the commnad and 4 bytes for the key
        let mut header_buf = [0; 5];

        match stream.read_exact(&mut header_buf) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                //Client disconnected cleanly
            }
            Err(e) => {
                eprintln!("I/O Error in client connection {}", e);
            }
        }

        let command = Command::try_from(header_buf[0]).unwrap();
        let key_len =
            u32::from_be_bytes([header_buf[1], header_buf[2], header_buf[3], header_buf[4]]);

        if key_len > MAX_KEY_SIZE {
            let _ = stream.write_all(b"ERR: key too large");
            break;
        }

        let mut key = vec![0u8; key_len as usize];

        if stream.read_exact(&mut key).is_err() {
            break;
        }

        let _ = match command {
            Command::Get => {
                if handle_get(&mut stream, &key, &cache).is_err() {
                    break;
                }
            }
            Command::Set => {
                if handle_set(&mut stream, &key, &cache).is_err() {
                    break;
                }
            }
            _ => {
                let _ = stream.write_all(b"ERR: Handling Unknown command");
            }
        };
    }
}

fn handle_get(stream: &mut TcpStream, key: &[u8], cache: &Cache) -> Result<(), &'static str> {
    //Read lock
    let map = match cache.read() {
        Ok(map) => map,
        Err(posioned) => {
            eprintln!("Poisoned guard");
            posioned.into_inner()
        }
    };

    match map.get(key) {
        Some(value) => {
            let value_len: u32 = value.len().try_into().expect("Value exceed u32 max");
            let mut response = Vec::with_capacity(4 + value.len());
            response.extend_from_slice(&value_len.to_be_bytes());
            response.extend_from_slice(&value);
            let _ = stream.write_all(&response);
            Ok(())
        }
        None => {
            let _ = stream.write_all(&0u32.to_be_bytes());
            Ok(())
        }
    }
}
fn handle_set(stream: &mut TcpStream, key: &[u8], cache: &Cache) -> Result<(), &'static str> {
    let mut val_header = [0u8; 4];
    if stream.read_exact(&mut val_header).is_err() {
        return Err("Failed to read");
    }

    let value_len = u32::from_be_bytes(val_header);
    if value_len > MAX_VALUE_SIZE {
        return Err("Value greater than max size");
    }

    let mut value = vec![0u8; value_len as usize];
    if stream.read_exact(&mut value).is_err() {
        return Err("Failed to read value");
    }

    //Scoped write lock
    let mut map = cache.write().unwrap();
    map.insert(key.to_vec(), value);
    let _ = stream.write_all(b"STORED\n");
    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let cache: Cache = Arc::new(RwLock::new(HashMap::with_capacity(10_000)));

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
