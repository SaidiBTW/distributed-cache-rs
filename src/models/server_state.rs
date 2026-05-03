use crate::{models::cache_store::CacheStore, raft::Node};

pub struct ServerState {
    pub map: CacheStore,
    pub node: Node,
    pub peers: Vec<String>,
}

impl ServerState {
    pub fn new(cache_capacity: usize, node_id: u32) -> ServerState {
        ServerState {
            map: CacheStore::new(cache_capacity),
            node: Node::new(node_id),
            peers: vec![
                String::from("127.0.0.1:7878"),
                String::from("127.0.0.1:7879"),
                String::from("127.0.0.1:7880"),
            ],
        }
    }
}
