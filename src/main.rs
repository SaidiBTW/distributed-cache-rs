use std::{
    collections::HashMap,
    env,
    io::{self, BufReader, BufWriter, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, RwLock,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cache::{
    cache_store::{Cache, CacheStore, ServerState},
    event::Event,
    raft::{Node, NodeStatus},
    status::Status,
    thread_pool::ThreadPool,
};

use cache::command::Command;

const MAX_KEY_SIZE: u32 = 1024; // 1 KB
const MAX_VALUE_SIZE: u32 = 1024 * 1024; //1MB

fn handle_client_event(stream: &TcpStream) {
    //Set a timeout to prevent blocking forever
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
        eprintln!("Failed to set read timeout {}", e);
    }
    let read_stream = stream.try_clone().expect("Error creating read stream");
    let write_stream = stream.try_clone().expect("Error creating write stream");

    let mut reader = BufReader::with_capacity(8192, read_stream);
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
                if handle_get(&mut writer, &key).is_err() {
                    break;
                }
            }
            Command::Set => {
                if handle_set(&mut writer, &mut reader, &key).is_err() {
                    break;
                }
            }
            Command::Del => {
                if handle_delete(&mut writer, &key).is_err() {
                    break;
                }
            }
            Command::Heartbeat => {
                todo!()
            }
            _ => {
                break;
            }
        };
        if writer.flush().is_err() {
            break;
        }
    }
}

// fn handle_client(stream: TcpStream, cache: &mut CacheStore) {
//     //Set a timeout to prevent blocking forever
//     if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
//         eprintln!("Failed to set read timeout {}", e);
//     }
//     let write_stream = stream.try_clone().expect("Error creating write stream");

//     let mut reader = BufReader::with_capacity(8192, stream);
//     let mut writer = BufWriter::with_capacity(8192, write_stream);
//     loop {
//         //Using 1 byte for the commnad and 4 bytes for the key
//         let mut header_buf = [0; 5];

//         match reader.read_exact(&mut header_buf) {
//             Ok(_) => {}
//             Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
//                 //Client disconnected cleanly
//             }
//             Err(e) => {
//                 eprintln!("I/O Error in client connection {}", e);
//             }
//         }

//         let command = Command::try_from(header_buf[0]).unwrap();
//         let key_len =
//             u32::from_be_bytes([header_buf[1], header_buf[2], header_buf[3], header_buf[4]]);

//         if key_len > MAX_KEY_SIZE {
//             let _ = writer.write_all(b"ERR: key too large");
//             break;
//         }

//         let mut key = vec![0u8; key_len as usize];

//         if reader.read_exact(&mut key).is_err() {
//             break;
//         }

//         let _ = match command {
//             Command::Get => {
//                 if handle_get(&mut writer, &key, &cache).is_err() {
//                     break;
//                 }
//             }
//             Command::Set => {
//                 if handle_set(&mut writer, &mut reader, &key, cache).is_err() {
//                     break;
//                 }
//             }
//             Command::Del => {
//                 if handle_delete(&mut writer, &key, cache).is_err() {
//                     break;
//                 }
//             }
//             Command::Heartbeat => {
//                 todo!()
//             }
//             _ => {
//                 break;
//             }
//         };
//         if writer.flush().is_err() {
//             break;
//         }
//     }
// }

fn handle_get(
    writer: &mut BufWriter<TcpStream>,
    key: &[u8],
    // cache: &Cache,
) -> Result<(), &'static str> {
    write_response(writer, Status::Ok, b"value");
    Ok(())
    // match cache.get(key) {
    //     Some(value) => {
    //         let _ = write_response(writer, Status::Ok, value);

    //         Ok(())
    //     }
    //     None => {
    //         let _ = write_response(writer, Status::NotFound, b"");

    //         Ok(())
    //     }
    // }
}

fn handle_delete(
    writer: &mut BufWriter<TcpStream>,
    key: &[u8],
    // cache: &mut Cache,
) -> io::Result<()> {
    //     match cache.delete(key) {
    //         true => write_response(writer, Status::Ok, b""),
    //         false => write_response(writer, Status::NotFound, b""),
    //     }
    Ok(())
}

fn handle_heartbeat(reader: &mut BufReader<TcpStream>, node: &Node) -> io::Result<()> {
    node.refresh_heartbeat();
    Ok(())
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
    // cache: &mut Cache,
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

    // //Scoped write lock
    // let server_state = cache;
    // server_state.set(key.to_vec(), &value).expect("Error OOM");
    let _ = write_response(writer, Status::Ok, b"");
    Ok(())
}

fn init() {
    let node_ip = String::from(env::var("BASE_URL").expect("Base Ip is not set"));
}

fn main() {
    // Initialize
    println!("Booting Distributed Cache Node...");

    let state = ServerState::new(10 * 1024 * 1024);

    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        run_state_machine(receiver, state);
    });

    let timer_tx = sender.clone();
    create_election_timer(timer_tx);

    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind port");
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                //Give each worker a clone of the sernder to tal to the state
                let worker_sender_channel = sender.clone();
                let mut thread_stream = stream.try_clone().unwrap();
                pool.execute(move || {
                    // handle_client(stream, cache_ref, &node);
                    let (reply_tx, reply_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
                        mpsc::channel();
                    // handle_client(thread_stream, &state.map);

                    worker_sender_channel
                        .send(Event::ClientCommand {
                            stream: thread_stream,
                            reply_to: reply_tx,
                        })
                        .unwrap();

                    if let Ok(response) = reply_rx.recv() {
                        let _ = stream.write_all(&response);
                        println!("{:?}", response);
                    }
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
}

fn run_state_machine(receiver: mpsc::Receiver<Event>, mut state: ServerState) {
    let mut last_heartbeat = Instant::now();
    let mut current_timeout = Duration::from_millis(generate_random_number());
    while let Ok(event) = receiver.recv() {
        println!("Received event {:?}", event);
        match event {
            Event::ElectionTimeout => {
                if last_heartbeat.elapsed() >= current_timeout {
                    if state.node.state != NodeStatus::Leader {
                        state.node.state = NodeStatus::Candidate;
                        state.node.current_term += 1;
                        state.node.voted_for = Some(0);

                        last_heartbeat = Instant::now();
                        //Request vote RPC
                    }
                } else {
                    println!(
                        "The current node state is {:?} and the term is {},",
                        state.node.state, state.node.current_term
                    );
                }
            }

            Event::HeartbeartTick => {
                if state.node.state == NodeStatus::Leader {
                    //Broadcase append entries to peers
                    todo!()
                }
            }
            Event::RpcMessage(items) => {
                // handing incoming Raft Messages from other nodes
            }
            Event::ClientCommand { stream, reply_to } => {
                handle_client_event(&stream);
                reply_to.send([0u8; 9].to_vec()).unwrap();
                break;
            }
        }
    }
}

fn create_election_timer(tx: mpsc::Sender<Event>) {
    thread::spawn(move || {
        loop {
            let timeout = generate_random_number();

            thread::sleep(Duration::from_millis(timeout));

            println!("Timeout ended sending event");
            if tx.send(Event::ElectionTimeout).is_err() {
                break;
            }
        }
    });
}

fn generate_random_number() -> u64 {
    //generate a random number between 150-200 to use for timeouts
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    150 + (nanos % 51) as u64
}
