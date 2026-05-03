use crate::raft::LogEntry;

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

#[derive(Debug, Clone)]
pub struct AppendEntriesArgs {
    pub term: u64,
    pub leader_id: u32,
    pub prev_log_index: u32,
    pub prev_log_term: u32,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u32,
}

#[derive(Debug, Clone)]
pub struct AppendEntriesReply {
    pub term: u64,
    pub success: bool,
}
