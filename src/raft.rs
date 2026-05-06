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
    command,
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
    pub node_status: NodeStatus,
    pub voted_for: Option<u32>,
    pub current_term: u64,
    pub commit_index: u64,
    pub leader: Option<String>,
    pub log: Log,
}

#[derive(Debug, Clone)]
pub struct Log {
    pub entries: Vec<LogEntry>,
}

impl Log {
    pub fn new() -> Log {
        Log {
            entries: Vec::with_capacity(20),
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes_repr = vec![];

        let log_len = self.entries.len() as u32;
        let log_len = log_len.to_be_bytes();

        bytes_repr.extend_from_slice(&log_len);

        for entry in &self.entries {
            bytes_repr.extend_from_slice(&entry.to_bytes());
        }

        println!("Log to bytes has {}", bytes_repr.len());

        bytes_repr
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Log {
        let mut entries = vec![];
        let log_len = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let mut pointer: usize = 4; //accounting for log len
        println!("Byte len {}", bytes.len());
        println!("Log len {}", log_len);

        if log_len > 0 {
            for iter in 0..log_len as usize {
                let command_len =
                    u32::from_be_bytes(bytes[pointer..(pointer + 4)].try_into().unwrap());
                println!("Command Len {command_len}");
                let end_of_command: usize =
                    (pointer + 8 + 4 + command_len as usize).try_into().unwrap(); // term + command_len + command
                println!("End of command is at index {end_of_command}");

                let log_entry = LogEntry::from_bytes(bytes[pointer..end_of_command].to_vec());
                entries.push(log_entry);
                pointer = end_of_command;
                println!("Pointer after {} iter", iter);
            }
        }

        Log { entries }
    }

    pub fn append_to_entries(&mut self, entries: Vec<LogEntry>) {
        self.entries.extend_from_slice(&entries);

        println!("Extending entries to len {:?}", self.entries);
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub command: Vec<u8>,
    pub term: u64,
}

impl LogEntry {
    pub fn to_bytes(&self) -> Vec<u8> {
        let term = self.term.to_be_bytes();
        let command_len = (self.command.len() as u32).to_be_bytes();

        let mut bytes_repr = vec![];

        bytes_repr.extend_from_slice(&command_len);
        bytes_repr.extend_from_slice(&term);
        bytes_repr.extend_from_slice(&self.command);
        println!("{:?}", self);
        println!("Command len : {:?}", self.command.len());

        println!("Log entry is repr by {} bytes", bytes_repr.len());

        bytes_repr
    }

    pub fn from_bytes(bytes: Vec<u8>) -> LogEntry {
        let term: u64 = u64::from_be_bytes(bytes[4..12].try_into().unwrap());

        let command = bytes[12..bytes.len()].try_into().unwrap();

        LogEntry { command, term }
    }
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
            voted_for: None,
            node_status: NodeStatus::Follower,
            commit_index: 0,
            current_term: 0,
            leader: None,
            log: Log::new(),
        };

        // node.setup_timeout(timeout);

        node
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
        self.node_status = NodeStatus::Leader;
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

#[cfg(test)]
pub mod tests {

    use crate::rpc::AppendEntriesArgs;

    use super::*;

    #[test]
    pub fn test_log_serialization() {
        let log = Log {
            entries: vec![LogEntry {
                command: vec![1u8],

                term: 3,
            }],
        };

        let log_bytes = log.to_bytes();

        println!("Log bytes in len {}", log_bytes.len());

        let log_deserialized = Log::from_bytes(log_bytes);

        println!("Log deserialized {:?}", log_deserialized);

        assert_eq!(log.entries.len(), log_deserialized.entries.len());
    }

    #[test]
    pub fn test_log_entry_serialization() {
        let log_entry = LogEntry {
            command: vec![2u8, 4u8],
            term: 1,
        };

        let bytes = log_entry.to_bytes();

        let log_entry_deserialized = LogEntry::from_bytes(bytes);

        println!("{:?}", log_entry_deserialized);

        assert_eq!(
            log_entry.command.len(),
            log_entry_deserialized.command.len()
        );
    }

    #[test]
    pub fn test_append_entries_serialization() {
        let mut log = Log::new();
        log.add_entry(LogEntry {
            command: vec![0u8],
            term: 3,
        });
        let append_entries = AppendEntriesArgs {
            leader_commit: 0,
            term: 1,
            leader_id: 1,
            prev_log_index: 0,
            prev_log_term: 0,
            log: log,
        };

        println!("Log serialized {:?}", append_entries);

        let bytes_repr = append_entries.to_bytes();

        println!("Bytes len {}", bytes_repr.len());

        let append_entries_deserialized = AppendEntriesArgs::from_bytes(bytes_repr);

        println!("Log deserialized {:?}", append_entries_deserialized);

        assert!(true)
    }
}
