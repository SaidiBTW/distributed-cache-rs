use std::{net::TcpStream, sync::mpsc::Sender};

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
    RpcMessage(Vec<u8>),
}
