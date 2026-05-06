use std::{net::TcpStream, sync::mpsc::Sender};

use crate::rpc::{AppendEntriesArgs, RequestVoteArgs};

#[derive(Debug)]
pub enum Event {
    // Covers SET/GET/DELETE
    ClientCommand {
        command: Vec<u8>,
        reply_to: Sender<Vec<u8>>,
    },

    //We havent heard from the leader, start and election
    ElectionTimeout,

    //RequestVote, AppendEntries RPXs
    IncomingRequestVote {
        args: RequestVoteArgs,
        reply_to: Sender<Vec<u8>>,
    },
    AppendEntries,
    IncomingAppendEntries {
        args: AppendEntriesArgs,
        reply_to: Sender<Vec<u8>>,
    },

    RpcReply {
        term: u64,
        vote_granted: bool,
    },
}
