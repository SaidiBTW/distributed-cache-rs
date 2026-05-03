use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use cache::raft::Node;

fn main() {
    let mut node = Node::new("Node A");
    let listener = TcpListener::bind(format!("{}", "127.0.0.1:7878")).unwrap();

    loop {
        let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
        let mut write_buf = vec![0u8; 9];

        write_buf.push(6u8);
        write_buf.extend_from_slice(&(0 as u32).to_be_bytes());
        write_buf.extend_from_slice(&(0 as u32).to_be_bytes());

        let _ = stream.write_all(&write_buf);

        thread::sleep(Duration::from_secs(2));
    }
}
