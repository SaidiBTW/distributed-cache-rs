use core::panic;
use std::{
    collections::HashMap,
    env,
    fs::read,
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
    event::Event,
    models::{
        cache_store::{Cache, CacheStore},
        server_state::ServerState,
        thread_pool::ThreadPool,
    },
    raft::{Node, NodeStatus, VoteRequest},
    rpc::{AppendEntriesArgs, AppendEntriesReply, RequestVoteArgs},
    status::Status,
};

use cache::rpc::RequestVoteReply;

use cache::command::Command;

const MAX_KEY_SIZE: u32 = 1024; // 1 KB
const MAX_VALUE_SIZE: u32 = 1024 * 1024; //1MB

fn handle_client_event(stream: &TcpStream, tx: Sender<Event>) {
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
            Command::RequestVote => {
                handle_request_vote(&mut writer, &mut reader, tx.clone());
            }
            Command::AppendEntries => {
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

fn handle_request_vote(
    writer: &mut BufWriter<TcpStream>,
    reader: &mut BufReader<TcpStream>,
    tx: mpsc::Sender<Event>,
) {
    let mut request = [0u8; 28];
    reader
        .read_exact(&mut request)
        .expect("Failed to read 28 bytes required by Request vote args.");

    let args = RequestVoteArgs::from_bytes(&request);

    //Create a channel to get reply from the state machine
    let (reply_tx, reply_rx) = mpsc::channel();

    let _ = tx.send(Event::IncomingRequestVote {
        args,
        reply_to: reply_tx,
    });
    println!("Locking awaiting response from state machine");

    if let Ok(reply_bytes) = reply_rx.recv() {
        let reply_bytes: [u8; 9] = reply_bytes.try_into().expect("Error parsing reply");
        let response = RequestVoteReply::from_bytes(&reply_bytes);
        let _ = writer.write_all(&reply_bytes);
    }
}

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
    //     match cache.delete(key) {unlo
    //         true => write_response(writer, Status::Ok, b""),
    //         false => write_response(writer, Status::NotFound, b""),
    //     }
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

fn main() {
    let node_id: u32 = env::var("NODE_ID").unwrap().parse().unwrap();
    let base_url = env::var("BASE_URL").unwrap();
    // Initialize
    println!("Booting Distributed Cache Node...");

    let state = ServerState::new(10 * 1024 * 1024, node_id);

    let (sender, receiver) = mpsc::channel();

    let (refresh_timer, refresh_timer_receiver) = mpsc::channel();

    let (heart_beat_timer, heart_beat_receiver) = mpsc::channel();

    thread::spawn(move || {
        run_state_machine(receiver, state, refresh_timer);
    });

    let timer_tx = sender.clone();
    let heartbeat_tx = sender.clone();
    create_election_timer(timer_tx, refresh_timer_receiver);
    create_heartbeat_timer(heartbeat_tx, heart_beat_receiver);

    let listener = TcpListener::bind(base_url).expect("Failed to bind port");
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                //Give each worker a clone of the sender to cpmminocate to the state
                let worker_sender_channel = sender.clone();

                pool.execute(move || {
                    let write_stream = stream.try_clone().expect("Error creating write stream");

                    let mut reader = BufReader::with_capacity(8192, stream);
                    let mut writer = BufWriter::with_capacity(8192, write_stream);

                    let mut command_buf = [0u8; 1];

                    reader
                        .read_exact(&mut command_buf)
                        .expect("Error reading command byte");

                    let command: Command =
                        Command::try_from(command_buf[0]).expect("Unexpected command");

                    println!("{:?} Command", command);

                    match command {
                        Command::Get => {
                            // handle_get(writer, key)
                        }
                        Command::Set => {}
                        Command::Del => {}
                        Command::RequestVote => {
                            handle_request_vote(&mut writer, &mut reader, worker_sender_channel);
                        }
                        Command::AppendEntries => {
                            handle_append_entries(&mut writer, &mut reader, worker_sender_channel);
                        }
                        _ => panic!("Unexpected value"),
                    };
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
fn handle_append_entries(
    writer: &mut BufWriter<TcpStream>,
    reader: &mut BufReader<TcpStream>,
    tx: mpsc::Sender<Event>,
) {
    let mut append_entries_buf = [0u8; 24];

    reader.read_exact(&mut append_entries_buf);

    let append_entries = AppendEntriesArgs::from_bytes(&append_entries_buf);

    let (append_tx, append_rx) = mpsc::channel();

    tx.send(Event::IncomingAppendEntries {
        args: append_entries,
        reply_to: append_tx,
    })
    .unwrap();

    if let Ok(value) = append_rx.recv() {
        let _ = writer.write_all(&value);
        println!(
            "Response - for upcoing append entries:{}",
            String::from_utf8_lossy(&value)
        );
    }
}
fn run_state_machine(
    receiver: mpsc::Receiver<Event>,
    mut state: ServerState,
    refresh_timer_sender: mpsc::Sender<Event>,
) {
    while let Ok(event) = receiver.recv() {
        // println!("Received event {:?}", event);
        match event {
            Event::ElectionTimeout => {
                if state.node.state != NodeStatus::Leader {
                    println!("Election timeout");
                    state.node.state = NodeStatus::Candidate;
                    state.node.current_term += 1;
                    state.node.voted_for = Some(0);

                    //Request vote RPC

                    let request_vote_args = RequestVoteArgs {
                        candidate_id: state.node.id,
                        term: state.node.current_term,
                        last_log_index: state.node.commit_index as usize,
                        last_log_term: state.node.current_term,
                    };

                    let node_ip = state.node.node_ip.clone();

                    let peers: Vec<String> = state
                        .peers
                        .clone()
                        .into_iter()
                        .filter(|x| *x != node_ip)
                        .collect();
                    let peerCount = peers.len();

                    let mut error_count = 0;
                    let quota = (peerCount / 2) + 1;
                    let mut accepted = 0;

                    for peer in peers.iter() {
                        let request_vote = RequestVoteArgs {
                            candidate_id: state.node.id,
                            term: state.node.current_term,
                            last_log_index: state.node.commit_index as usize,
                            last_log_term: state.node.current_term,
                        };
                        println!("Connecting to peer, {}", peer);
                        let stream = TcpStream::connect(peer);

                        match stream {
                            Ok(mut stream) => {
                                let mut write_buf = vec![4u8];
                                write_buf.extend_from_slice(&request_vote.to_bytes().unwrap());

                                stream.write_all(&mut write_buf).expect("Error writing ");

                                let mut read_buf = [0u8; 9];

                                stream.read_exact(&mut read_buf).unwrap();

                                // let request_response = RequestVoteReply::
                                let request_response = RequestVoteReply::from_bytes(&read_buf);

                                println!(
                                    "Response gotten from response buf {:?}",
                                    request_response
                                );
                                if request_response.vote_granted {
                                    accepted += 1;
                                }
                            }
                            Err(err) => {
                                error_count += 1;
                                println!("Error connecting to one of the nodes");
                            }
                        }
                    }

                    if accepted >= (quota) {
                        println!(
                            "Node {} should be leader has {} votes in term {} errors {} ",
                            state.node.id, accepted, request_vote_args.term, error_count
                        );
                        state.node.become_leader();
                    }
                    // Node::send_request_vote(
                    //     String::from("127.0.0.1:7878"),
                    //     request_vote_args,
                    //     tx,
                    // );
                }
            }

            Event::ClientCommand { stream, reply_to } => todo!(),
            Event::IncomingRequestVote { args, reply_to } => {
                println!("{:?}", args);
                if args.term <= state.node.current_term {
                    reply_to
                        .send(
                            RequestVoteReply {
                                term: args.term,
                                vote_granted: false,
                            }
                            .to_bytes()
                            .unwrap(),
                        )
                        .expect("Client closed");
                }
                // if args.last_log_index < (state.node.commit_index as usize) {
                //     reply_to
                //         .send(
                //             RequestVoteReply {
                //                 term: args.term,
                //                 vote_granted: false,
                //             }
                //             .to_bytes()
                //             .unwrap(),
                //         )
                //         .expect("Client closed");
                // }

                //Else we vote
                state.node.voted_for = Some(args.candidate_id);
                // state.node.current_term = args.term;
                //we have voted for the candidate
                println!("voted for {}", args.candidate_id);

                reply_to
                    .send(
                        RequestVoteReply {
                            term: args.term,
                            vote_granted: true,
                        }
                        .to_bytes()
                        .unwrap(),
                    )
                    .unwrap();
            }

            Event::RpcReply { term, vote_granted } => todo!(),
            Event::AppendEntries => {
                if state.node.state == NodeStatus::Leader {
                    println!("I am leader sending heartbeart");
                    let node_ip = state.node.node_ip.clone();

                    let peers: Vec<String> = state
                        .peers
                        .clone()
                        .into_iter()
                        .filter(|x| *x != node_ip)
                        .collect();

                    for peer in peers {
                        println!("Appending entries to {}", peer);

                        let stream = TcpStream::connect(peer);

                        match stream {
                            Ok(mut stream) => {
                                let mut write_buf = vec![5u8]; //Append Entries command

                                write_buf.extend_from_slice(
                                    &AppendEntriesArgs {
                                        entries: vec![],
                                        leader_commit: state.node.commit_index as u32,
                                        leader_id: state.node.id,
                                        prev_log_index: state.node.commit_index as u32,
                                        prev_log_term: state.node.current_term as u32,
                                        term: state.node.current_term,
                                    }
                                    .to_bytes(),
                                );

                                stream.write_all(&write_buf).unwrap();

                                let mut response_buf = [0u8; 9];

                                stream.read_exact(&mut response_buf);

                                let response = AppendEntriesReply::from_bytes(&response_buf);

                                if response.term > state.node.current_term && response.success {
                                    println!("Has a greater term than me i should follow");
                                    state.node.state = NodeStatus::Follower;
                                    state.node.current_term = response.term;
                                } else {
                                }
                            }
                            Err(_) => {
                                println!("Should retry")
                            }
                        }
                    }
                } else {
                    // Skip event if you are not leader
                }
            }
            Event::IncomingAppendEntries { args, reply_to } => {
                println!("{:?}", args);
                if args.term < state.node.current_term {
                    println!("You are a higher term that the leader reject offer");
                    reply_to
                        .send(
                            AppendEntriesReply {
                                success: false,
                                term: state.node.current_term,
                            }
                            .to_bytes(),
                        )
                        .unwrap();
                }
                if args.term > state.node.current_term {
                    reply_to
                        .send(
                            AppendEntriesReply {
                                success: true,
                                term: args.term,
                            }
                            .to_bytes(),
                        )
                        .unwrap();
                }

                refresh_timer_sender.send(Event::AppendEntries);
            }
        }
    }
}

fn create_heartbeat_timer(tx: mpsc::Sender<Event>, refresh_timer_receiver: mpsc::Receiver<Event>) {
    thread::spawn(move || {
        loop {
            match refresh_timer_receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(_) => {
                    //Interrupt received stop sending hearbeats
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    //No interrupt receved continue sending heart beats
                    tx.send(Event::AppendEntries).unwrap();
                }
            };
        }
    });
}

fn create_election_timer(tx: mpsc::Sender<Event>, refresh_timer_receiver: mpsc::Receiver<Event>) {
    thread::spawn(move || {
        let node_id = env::var("NODE_ID").unwrap();
        let timeout = generate_random_number();
        let mut deadline = Instant::now() + Duration::from_millis(timeout);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());

            match refresh_timer_receiver.recv_timeout(remaining) {
                Ok(_) => {
                    deadline = Instant::now() + Duration::from_millis(timeout);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if tx.send(Event::ElectionTimeout).is_err() {
                        break;
                    }
                    deadline = Instant::now() + Duration::from_millis(timeout); // reset for next round
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });
}

fn generate_random_number() -> u64 {
    //generate a random number between 1500-2000 to use for timeouts
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    1500 + (nanos % 501) as u64
}
