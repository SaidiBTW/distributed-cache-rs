use std::{net::TcpStream, sync::mpsc::Sender};

use crate::rpc::RequestVoteArgs;

#[derive(Debug)]
pub enum Event {
    // Covers SET/GET/DELETE
    ClientCommand {
        stream: TcpStream,
        reply_to: Sender<Vec<u8>>,
    },
    //Sending heartbeats to followers
    HeartbeartTick,
    //We havent heard from the leader, start and election
    ElectionTimeout,

    //RequestVote, AppendEntries RPXs
    IncomingRequestVote {
        args: RequestVoteArgs,
        reply_to: Sender<Vec<u8>>,
    },
    IncomingAppendEntries {
        args: (),
        reply_to: Sender<Vec<u8>>,
    },

    RpcReply {
        term: u64,
        vote_granted: bool,
    },
}
