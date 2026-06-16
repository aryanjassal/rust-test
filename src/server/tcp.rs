use std::{
    io::{self, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Sender},
    thread,
};

use crate::redis::{
    parser,
    protocol::{Request, Response},
    resp,
};

pub fn run(addr: &str, core_tx: Sender<Request>) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let stream = stream?;
        let tx = core_tx.clone();

        thread::spawn(move || {
            if let Err(error) = handle_client(stream, tx) {
                eprintln!("client error: {error}");
            }
        });
    }
    Ok(())
}

fn handle_client(stream: TcpStream, core_tx: Sender<Request>) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    loop {
        let parts = match resp::read_command(&mut reader) {
            Ok(parts) => parts,
            Err(error) => {
                resp::write_response(&mut writer, Response::Error(error))?;
                writer.flush()?;
                break;
            }
        };

        let command = match parser::parse_command(&parts) {
            Ok(command) => command,
            Err(error) => {
                resp::write_response(&mut writer, Response::Error(error))?;
                writer.flush()?;
                continue;
            }
        };

        let (reply_tx, reply_rx) = mpsc::channel();
        if core_tx
            .send(Request {
                command,
                reply: reply_tx,
            })
            .is_err()
        {
            resp::write_response(&mut writer, Response::Error("core unavailable".to_string()))?;
            writer.flush()?;
            break;
        }

        let response = match reply_rx.recv() {
            Ok(response) => response,
            Err(_) => Response::Error("core unavailable".to_string()),
        };
        resp::write_response(&mut writer, response)?;
        writer.flush()?;
    }

    Ok(())
}
