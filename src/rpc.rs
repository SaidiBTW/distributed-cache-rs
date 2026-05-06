use crate::raft::{Log, LogEntry};

pub enum RaftRpc {
    RequestVote {},
}

#[derive(Debug, Clone)]
pub struct RequestVoteArgs {
    pub term: u64,
    pub candidate_id: u32,
    pub last_log_index: usize,
    pub last_log_term: u64,
}

impl RequestVoteArgs {
    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        let mut buffer: Vec<u8> = vec![];

        buffer.extend_from_slice(&self.term.to_be_bytes());
        buffer.extend_from_slice(&self.candidate_id.to_be_bytes());
        buffer.extend_from_slice(&(self.last_log_index as u64).to_be_bytes());
        buffer.extend_from_slice(&self.last_log_term.to_be_bytes());

        Ok(buffer)
    }

    pub fn from_bytes(bytes: &[u8; 28]) -> RequestVoteArgs {
        if bytes.len() != 28 {
            panic!(
                "Unexpected payload does not equal required len {}",
                bytes.len()
            );
        }
        //4 bytes for term
        let term = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        //8 bytes for candidate id
        let candidate_id = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let last_log_index = u64::from_be_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19],
        ]);

        let last_commit_index = u64::from_be_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23], bytes[24], bytes[25], bytes[26], bytes[27],
        ]);

        RequestVoteArgs {
            term: term,
            candidate_id: candidate_id,
            last_log_index: last_log_index as usize,
            last_log_term: last_commit_index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestVoteReply {
    pub term: u64,
    pub vote_granted: bool,
}

impl RequestVoteReply {
    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        //1 byte for either accept od reject
        // 8 bytes for the current term

        let mut bytes_repr = vec![];

        if self.vote_granted {
            bytes_repr.push(1u8);
        } else {
            bytes_repr.push(0u8);
        };

        bytes_repr.extend_from_slice(&self.term.to_be_bytes());
        Ok(bytes_repr)
    }

    pub fn from_bytes(bytes: &[u8; 9]) -> RequestVoteReply {
        let vote = u8::from_be_bytes(bytes[0..1].try_into().unwrap());
        let vote_granted = vote == 1;
        let term = u64::from_be_bytes(bytes[1..9].try_into().unwrap());

        RequestVoteReply { term, vote_granted }
    }
}

#[derive(Debug)]
pub struct AppendEntriesArgs {
    pub term: u64,
    pub leader_id: u32,
    pub prev_log_index: u32,
    pub prev_log_term: u32,
    pub log: Log,
    pub leader_commit: u32,
}

impl AppendEntriesArgs {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes_repr = vec![];
        bytes_repr.extend_from_slice(&self.leader_id.to_be_bytes());
        bytes_repr.extend_from_slice(&self.term.to_be_bytes());
        bytes_repr.extend_from_slice(&self.leader_commit.to_be_bytes());
        bytes_repr.extend_from_slice(&self.prev_log_term.to_be_bytes());
        bytes_repr.extend_from_slice(&self.prev_log_index.to_be_bytes());
        bytes_repr.extend_from_slice(&self.log.to_bytes());

        bytes_repr
    }

    pub fn from_bytes(bytes: Vec<u8>) -> AppendEntriesArgs {
        println!("From bytes has {}. len", bytes.len());
        AppendEntriesArgs {
            leader_id: u32::from_be_bytes(bytes[0..4].try_into().unwrap()),
            term: u64::from_be_bytes(bytes[4..12].try_into().unwrap()),
            leader_commit: u32::from_be_bytes(bytes[12..16].try_into().unwrap()),
            prev_log_term: u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            log: Log::from_bytes(bytes[24..].try_into().unwrap()),
            // log: Log::new(),
            prev_log_index: u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppendEntriesReply {
    pub conflict_index: u64,
    pub conflict_term: Option<u64>,
    pub success: bool,
}

impl AppendEntriesReply {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes_repr = vec![];
        if self.success {
            bytes_repr.push(1u8);
        } else {
            bytes_repr.push(0u8);
        };

        match self.conflict_term {
            Some(term) => {
                bytes_repr.push(1u8);
                bytes_repr.extend_from_slice(&term.to_be_bytes());
            }
            None => bytes_repr.push(0u8),
        }

        bytes_repr.extend_from_slice(&self.conflict_index.to_be_bytes());

        bytes_repr
    }

    pub fn from_bytes(bytes: Vec<u8>) -> AppendEntriesReply {
        let mut cursor: usize = 0;
        //get success bit
        cursor += 1;
        let success = u8::from_be_bytes(bytes[0..cursor].try_into().unwrap()) == 1;

        let conflict_option_existence_bit =
            u8::from_be_bytes(bytes[cursor..cursor + 1].try_into().unwrap());
        cursor += 1;
        let conflict_term = match conflict_option_existence_bit {
            0 => {
                cursor += 1;
                None
            }
            1 => {
                if cursor + 8 > bytes.len() {
                    panic!("Unexpected EOF")
                }
                let term = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());

                cursor += 8;

                Some(term)
            }
            _ => panic!("invalid option tag"),
        };

        let conflict_index = u64::from_be_bytes(bytes[cursor..cursor + 8].try_into().unwrap());

        AppendEntriesReply {
            conflict_index,
            conflict_term,
            success,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_append_entries_reply() {
        let append_entries_reply = AppendEntriesReply {
            success: true,
            conflict_index: 2,
            conflict_term: Some(20),
        };

        assert!(append_entries_reply.to_bytes().len() == 18);

        let repr = append_entries_reply.to_bytes();

        let append_entries_deserial = AppendEntriesReply::from_bytes(repr);

        println!("Reply {:?}", append_entries_deserial);

        assert!(true);
    }
}
