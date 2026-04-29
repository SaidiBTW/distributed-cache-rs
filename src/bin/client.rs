use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

#[repr(u8)]
#[derive(Debug, PartialEq)]
enum Status {
    Ok = 0x00,
    NotFound = 0x01,
    Err = 0xFF,
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Status::Ok),
            0x01 => Ok(Status::NotFound),
            0xFF => Ok(Status::Err),
            other => Err(other),
        }
    }
}

#[derive(Debug)]
enum Response {
    Ok(Vec<u8>),
    NotFound,
    Err(String),
}

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7878").expect("Error connecting to server");

    println!("Successfully connected to server");

    let key = b"Hello123";
    let value = b"Hello1234Value";

    let missing_key = b"Missing123";

    // Test the set command;

    match send_set(&mut stream, key, value) {
        Ok(Response::Ok(_)) => println!("Set successful"),
        Ok(Response::Err(error)) => {
            println!(
                "Error from server: Corresponding reads may fail {:#?}",
                error
            )
        }
        _ => println!("Unknown matching arm"),
    }

    match send_get(&mut stream, key) {
        Ok(Response::Ok(response)) => println!(
            "Response from server {:#?}",
            String::from_utf8_lossy(&response)
        ),
        Ok(Response::Err(error)) => println!("Error from server {:#?}", error),
        Ok(Response::NotFound) => println!(
            "Did not find key '{}' in server",
            String::from_utf8_lossy(key)
        ),
        Err(err) => println!("Error {:#?}", err),
    }

    //For missing key

    match send_get(&mut stream, missing_key) {
        Ok(Response::Ok(response)) => println!(
            "Response from server {:#?}",
            String::from_utf8_lossy(&response)
        ),
        Ok(Response::Err(error)) => println!("Error from server {:#?}", error),
        Ok(Response::NotFound) => println!(
            "Did not find key '{}' in server",
            String::from_utf8_lossy(missing_key)
        ),
        Err(err) => println!("Error {:#?}", err),
    }
}

fn read_response(stream: &mut TcpStream) -> io::Result<Response> {
    //Read the status
    let mut status_buf = [0u8; 1];
    stream.read_exact(&mut status_buf)?;

    //Read body length
    let mut body_buf = [0u8; 4];
    stream.read_exact(&mut body_buf)?;

    let body_len = u32::from_be_bytes(body_buf);

    //Read the body
    let mut body = vec![0u8; body_len as usize];
    if body_len > 0 {
        stream.read_exact(&mut body)?;
    }

    let response = match Status::try_from(status_buf[0]) {
        Ok(Status::Ok) => Response::Ok(body),
        Ok(Status::NotFound) => Response::NotFound,
        Ok(Status::Err) => Response::Err(String::from_utf8_lossy(&body).into_owned()),
        Err(unknown) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown status byte: {:#?}", unknown),
            ));
        }
    };
    Ok(response)
}

fn send_get(stream: &mut TcpStream, key: &[u8]) -> io::Result<Response> {
    let key_len = key.len() as u32;

    let mut frame = Vec::with_capacity(1 + 4 + key.len());

    frame.push(1u8); //Command bit for get
    frame.extend_from_slice(&key_len.to_be_bytes());
    frame.extend_from_slice(key);

    stream.write_all(&frame)?;
    read_response(stream)
}

fn send_set(stream: &mut TcpStream, key: &[u8], value: &[u8]) -> io::Result<Response> {
    let key_len = key.len() as u32;
    let value_len = value.len() as u32;

    let mut frame = Vec::with_capacity(1 + 4 + key.len() + 4 + value.len());

    frame.push(2u8);
    frame.extend_from_slice(&key_len.to_be_bytes());
    frame.extend_from_slice(key);

    frame.extend_from_slice(&value_len.to_be_bytes());
    frame.extend_from_slice(value);

    stream.write_all(&frame)?;
    read_response(stream)
}
