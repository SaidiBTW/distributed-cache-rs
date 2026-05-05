use core::time;
use std::{
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Condvar, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    vec,
};

use crate::{
    event::Event,
    rpc::{RequestVoteArgs, RequestVoteReply},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeStatus {
    Follower,
    Candidate,
    Leader,
}

pub struct Node {
    pub id: u32,
    pub node_ip: String,
    pub state: NodeStatus,
    pub voted_for: Option<u32>,
    pub current_term: u64,
    pub commit_index: u64,
    pub timeout: u64,
    pub leader: Option<String>,
}

pub struct Log {
    logs: Vec<LogEntry>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    command: Vec<u8>,
    index: u64,
    term: u64,
}

pub struct Vote;

pub struct VoteRequest {
    term: u32,
}

#[repr(u8)]
enum VoteRequestResult {
    Accepted = 0x00,
    Rejected = 0x01,
    VoteRequestError = 0xFF,
}

impl TryFrom<u8> for VoteRequestResult {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(VoteRequestResult::Accepted),
            0x01 => Ok(VoteRequestResult::Rejected),
            other => Err(other),
        }
    }
}

struct TimerState {
    deadline: Instant,
}

const HEARTBEAT_INTERVAL: u64 = 200;

impl Node {
    pub fn new(id: u32) -> Node {
        let node_ip = env::var("BASE_URL").expect("Node Base url not set");
        let timeout = generate_random_number() * 20;
        let node = Node {
            id: id,
            node_ip: node_ip,
            timeout: timeout,
            voted_for: None,
            state: NodeStatus::Follower,
            commit_index: 0,
            current_term: 0,
            leader: None,
        };

        // node.setup_timeout(timeout);

        node
    }

    pub fn send_heart_beat(&self) {
        let node_ip = self.node_ip.clone();
        let timeout = self.timeout;
        thread::spawn(move || {
            //This are the members of nodes in the cluster
            thread::sleep(Duration::from_millis(timeout));
            let MEMBERS: Vec<String> = vec![
                String::from("127.0.0.1:7878"),
                String::from("127.0.0.1:7879"),
                String::from("127.0.0.1:7880"),
            ];
            loop {
                thread::sleep(Duration::from_millis(HEARTBEAT_INTERVAL));
                println!("Sending heartbeat interval");
                let other_members: Vec<String> = MEMBERS
                    .clone()
                    .into_iter()
                    .filter(|x| *x != node_ip)
                    .collect();
                for member in other_members.iter() {
                    println!("Sending heart_beat to {}", member)
                }
            }
        });
    }

    pub fn ask_for_leadership(&self) {
        let node_ip = self.node_ip.clone();
        //This are the members of nodes in the cluster
        let MEMBERS: Vec<String> = vec![
            String::from("127.0.0.1:7878"),
            String::from("127.0.0.1:7879"),
            String::from("127.0.0.1:7880"),
        ];
        let other_members: Vec<String> = MEMBERS
            .clone()
            .into_iter()
            .filter(|x| *x != node_ip)
            .collect();
        // Make a request for being a leader
    }

    pub fn send_request_vote(peer_add: String, args: RequestVoteArgs, tx: mpsc::Sender<Event>) {
        thread::spawn(move || {
            let mut connection =
                TcpStream::connect(&peer_add).expect("Error connection to peer add");
            connection
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();

            let mut payload = vec![4u8];

            payload.extend_from_slice(&args.to_bytes().unwrap());

            let _ = connection.write_all(&payload);

            let mut response = [0u8; 9];

            connection.read_exact(&mut response).unwrap();

            // let request_vote_args = RequestVoteArgs::from_bytes(&response);

            // tx.send(Event::IncomingRequestVote {
            //     args: request_vote_args,
            //     reply_to: tx,
            // });

            todo!();
        });
    }

    pub fn become_leader(&mut self) {
        self.voted_for = Some(self.id);
        self.state = NodeStatus::Leader;
    }

    pub fn send_election_vote_request(&mut self) -> Result<(), &'static str> {
        //Send to other members in the cluster your current term and index
        let node_ip = self.node_ip.clone();
        //This are the members of nodes in the cluster
        let MEMBERS: Vec<String> = vec![
            String::from("127.0.0.1:7878"),
            String::from("127.0.0.1:7879"),
            String::from("127.0.0.1:7880"),
        ];

        let member_count = MEMBERS.len();
        let minimum_quota = (member_count / 2) + 1;
        let other_members: Vec<String> = MEMBERS
            .clone()
            .into_iter()
            .filter(|x| *x != node_ip)
            .collect();

        let mut positive_voters = 0;
        for member in other_members {
            let connection = TcpStream::connect(format!("http://{}", member));
            if let Err(e) = connection {
                eprintln!("Error occured when connection to {} : Error {}", member, e);
                continue;
            }

            let mut connection = connection.unwrap();
            let mut write_response: Vec<u8> = vec![];
            //9 Bytes -> 1 Command, 4 index bytes, 4 term byte
            write_response.push(7u8);
            let index = (self.commit_index as u32).to_be_bytes();
            let term = (self.current_term as u32).to_be_bytes();

            write_response.extend_from_slice(&index);
            write_response.extend_from_slice(&term);

            connection
                .write_all(&write_response)
                .expect("Error making request");

            let mut response_buf = [0; 1];

            connection
                .read_exact(&mut response_buf)
                .expect("Error reading result");

            let status = VoteRequestResult::try_from(response_buf[0]);

            match status {
                Ok(VoteRequestResult::Accepted) => {
                    positive_voters = positive_voters + 1;
                }
                _ => {
                    continue;
                }
            };
        }
        if positive_voters >= minimum_quota {
            self.leader = Some(String::from(node_ip));
        }

        Ok(())
    }

    pub fn handle_vote_request(&self, term: u64, index: u64) -> Result<(), &'static str> {
        if term < self.current_term {
            self.reject_vote();
            return Err("You are behind on terms. Node requesting leadership might be outdated.");
        }
        if index < self.commit_index {
            self.reject_vote();
            return Err("I have a more recent logs compared to you hence I am more updated");
        }

        self.accept_vote();
        Ok(())
    }

    fn reject_vote(&self) {}
    fn accept_vote(&self) {}
}

fn generate_random_number() -> u64 {
    //generate a random number between 1500-2000 to use for timeouts
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    1500 + (nanos % 501) as u64
}
