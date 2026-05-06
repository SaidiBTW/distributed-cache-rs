use std::{
    io::{self, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use crate::{response::Response, rpc::RequestVoteArgs, status::Status};

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl Client {
    pub fn connect(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;

        //Duplicate the stream at file descriptor level
        //Both handles refer to the same socket
        let writer_stream = stream.try_clone()?;

        Ok(Client {
            reader: BufReader::with_capacity(8192, stream),
            writer: BufWriter::with_capacity(8192, writer_stream),
        })
    }
}

impl Client {
    pub fn read_response(&mut self) -> io::Result<Response> {
        return Ok(Response::Ok(vec![]));
        let mut status_buf = [0u8; 1];
        self.reader.read_exact(&mut status_buf)?;

        let mut len_buf = [0u8; 4];
        self.reader.read_exact(&mut len_buf)?;
        let body_len = u32::from_be_bytes(len_buf) as usize;

        let mut body = vec![0u8; body_len];

        if body_len > 0 {
            self.reader.read_exact(&mut body)?;
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
}

impl Client {
    pub fn get(&mut self, key: &[u8]) -> io::Result<Response> {
        let key_len = key.len() as u32;

        self.writer.write_all(&[1u8])?;
        self.writer.write_all(&key_len.to_be_bytes())?;
        self.writer.write_all(key)?;

        self.writer.flush()?;

        self.read_response()
    }

    pub fn set(&mut self, key: &[u8], value: &[u8]) -> io::Result<Response> {
        let key_len = key.len() as u32;
        let value_len = value.len() as u32;

        self.writer.write_all(&[2u8])?;
        self.writer.write_all(&key_len.to_be_bytes())?;
        self.writer.write_all(key)?;

        self.writer.write_all(&value_len.to_be_bytes())?;
        self.writer.write_all(value)?;

        self.writer.flush()?;

        self.read_response()
    }

    pub fn delete(&mut self, key: &[u8]) -> io::Result<Response> {
        let key_len = key.len() as u32;
        self.writer.write_all(&[3u8])?;
        self.writer.write_all(&key_len.to_be_bytes())?;
        self.writer.write_all(key)?;

        self.writer.flush()?;

        self.read_response()
    }

    pub fn vote_request(&mut self) -> io::Result<Response> {
        let args = RequestVoteArgs {
            candidate_id: 1,
            last_log_term: 2,
            term: 3,
            last_log_index: 5,
        };
        self.writer.write_all(&[4u8])?;
        self.writer.write_all(&args.to_bytes().unwrap())?;

        println!("Buf Length: {}", self.writer.buffer().len());
        self.writer.flush()?;

        self.read_response()
    }
}
