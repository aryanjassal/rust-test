use std::{
    io::{self, BufReader, BufWriter, Write},
    net::TcpStream,
};

use crate::redis::{
    protocol::{Command, Response},
    resp,
};

pub struct TcpClient {
    writer: BufWriter<TcpStream>,
    reader: BufReader<TcpStream>,
}

impl TcpClient {
    pub fn connect(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream);
        Ok(Self { writer, reader })
    }

    pub fn send(&mut self, command: Command) -> Result<Response, String> {
        resp::write_command(&mut self.writer, command).map_err(|e| e.to_string())?;
        self.writer.flush().map_err(|e| e.to_string())?;
        resp::read_response(&mut self.reader)
    }
}
