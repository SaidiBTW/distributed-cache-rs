use std::{
    collections::HashMap,
    io::{self, BufReader, BufWriter, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use cache::{
    arena::{Arena, ArenaPtr},
    cache_store::CacheStore,
    status::Status,
    thread_pool::ThreadPool,
};

use cache::command::Command;

type Cache = Arc<RwLock<CacheStore>>;

const MAX_KEY_SIZE: u32 = 1024; // 1 KB
const MAX_VALUE_SIZE: u32 = 1024 * 1024; //1MB

fn handle_client(stream: TcpStream, cache: Cache) {
    //Set a timeout to prevent blocking forever
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
        eprintln!("Failed to set read timeout {}", e);
    }
    let write_stream = stream.try_clone().expect("Error creating write stream");

    let mut reader = BufReader::with_capacity(8192, stream);
    let mut writer = BufWriter::with_capacity(8192, write_stream);
    loop {
        //Using 1 byte for the commnad and 4 bytes for the key
        let mut header_buf = [0; 5];

        match reader.read_exact(&mut header_buf) {
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
            let _ = writer.write_all(b"ERR: key too large");
            break;
        }

        let mut key = vec![0u8; key_len as usize];

        if reader.read_exact(&mut key).is_err() {
            break;
        }

        let _ = match command {
            Command::Get => {
                if handle_get(&mut writer, &key, &cache).is_err() {
                    break;
                }
            }
            Command::Set => {
                if handle_set(&mut writer, &mut reader, &key, &cache).is_err() {
                    break;
                }
            }
            Command::Del => {
                if handle_delete(&mut writer, &key, &cache).is_err() {
                    break;
                }
            }
        };
        if writer.flush().is_err() {
            break;
        }
    }
}

fn handle_get(
    writer: &mut BufWriter<TcpStream>,
    key: &[u8],
    cache: &Cache,
) -> Result<(), &'static str> {
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
            let _ = write_response(writer, Status::Ok, value);

            Ok(())
        }
        None => {
            let _ = write_response(writer, Status::NotFound, b"");

            Ok(())
        }
    }
}

fn handle_delete(writer: &mut BufWriter<TcpStream>, key: &[u8], cache: &Cache) -> io::Result<()> {
    let mut map = cache.write().unwrap();

    match map.delete(key) {
        true => write_response(writer, Status::Ok, b""),
        false => write_response(writer, Status::NotFound, b""),
    }
}

fn write_response(
    writer: &mut BufWriter<TcpStream>,
    status: Status,
    body: &[u8],
) -> io::Result<()> {
    let body_len = body.len() as u32;
    writer.write_all(&[status as u8])?;
    writer.write_all(&body_len.to_be_bytes())?;
    writer.write_all(body)?;

    writer.flush()
}
fn handle_set(
    writer: &mut BufWriter<TcpStream>,
    reader: &mut BufReader<TcpStream>,
    key: &[u8],
    cache: &Cache,
) -> Result<(), &'static str> {
    let mut val_header = [0u8; 4];
    if reader.read_exact(&mut val_header).is_err() {
        return Err("Failed to read");
    }

    let value_len = u32::from_be_bytes(val_header);
    if value_len > MAX_VALUE_SIZE {
        return Err("Value greater than max size");
    }

    let mut value = vec![0u8; value_len as usize];
    if reader.read_exact(&mut value).is_err() {
        return Err("Failed to read value");
    }

    //Scoped write lock
    let mut map = cache.write().unwrap();
    map.set(key.to_vec(), &value).expect("Error OOM");
    let _ = write_response(writer, Status::Ok, b"");
    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let cache: Cache = Arc::new(RwLock::new(CacheStore::new(10_000)));

    let pool = ThreadPool::new(4);

    println!("Cache server listener on port 7878");

    for stream in listener.incoming().take(2) {
        match stream {
            Ok(stream) => {
                let cache_ref = Arc::clone(&cache);
                pool.execute(move || {
                    handle_client(stream, cache_ref);
                });
                // thread::spawn(move || {
                //     handle_client(stream, cache_ref);
                // });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    println!("Shutting down")
}
