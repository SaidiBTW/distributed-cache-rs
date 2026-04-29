use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7878").expect("Error connecting to server");

    println!("Successfully connected to server");

    let key = b"Hello123";
    let value = b"Hello1234Value";

    // Test the set command;

    let mut set_request = Vec::new();
    set_request.push(2u8); //Command 2-> Set

    // Key Length 4 bytes Big Endian + key
    set_request.extend_from_slice(&(key.len() as u32).to_be_bytes());
    set_request.extend_from_slice(key);

    // Value length
    set_request.extend_from_slice(&(value.len() as u32).to_be_bytes());
    set_request.extend_from_slice(value);

    //Send the value back
    stream.write_all(&set_request).unwrap();

    //Read the server acknowldgement STORED\n 7 bytes
    let mut response_buf = [0u8; 7];
    stream
        .read_exact(&mut response_buf)
        .expect("Error reading response");

    println!(
        "Server responded to SET {:?}",
        String::from_utf8_lossy(&response_buf).trim()
    );

    // Test the get command

    let mut get_request = Vec::new();
    get_request.push(1u8); //Command 1 -> Get

    //Key Lenght 4 bytes Big Endian + key
    get_request.extend_from_slice(&(key.len() as u32).to_be_bytes());
    get_request.extend_from_slice(key);

    //Send the value to server
    stream.write_all(&get_request).unwrap();

    //Read the value len of respinse response 4 bytes
    let mut val_len_buf = [0u8; 4];
    stream
        .read_exact(&mut val_len_buf)
        .expect("Failed to read get response");
    let value_len = u32::from_be_bytes(val_len_buf);

    if value_len == 0 {
        println!("Value not found this is a miss");
    } else {
        let mut val_response_buff = vec![0u8; value_len as usize];
        stream.read_exact(&mut val_response_buff).unwrap();
        println!(
            "Server responded with value-> {}",
            String::from_utf8_lossy(&val_response_buff)
        )
    }
}
