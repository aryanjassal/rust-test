mod cli;
mod redis;

use std::{
    io::{self, Write},
    sync::mpsc,
    thread,
};

fn main() {
    let (core_tx, core_rx) = mpsc::channel();

    thread::spawn(move || {
        let mut core = redis::core::DbCore::new("dump.rdb".to_string());
        core.load().unwrap();
        core.run(core_rx);
    });

    // Store the input commands
    let mut buffer = String::new();

    // REPL
    loop {
        // Read input
        buffer.clear();
        io::stdout().write_all("\n> ".as_bytes()).unwrap();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buffer).unwrap();
        let _ = buffer.pop().unwrap_or(' '); // Trim newline

        let parts = cli::split(buffer.trim())
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let command = match redis::parser::parse_command(&parts) {
            Ok(command) => command,
            Err(error) => {
                println!("{error}");
                continue;
            }
        };

        let (reply_tx, reply_rx) = mpsc::channel();
        if core_tx
            .send(redis::protocol::Request {
                command,
                reply: reply_tx,
            })
            .is_err()
        {
            println!("ERR core unavailable");
            break;
        }

        match reply_rx.recv() {
            Ok(response) => match response {
                redis::protocol::Response::Ok => println!("OK"),
                redis::protocol::Response::Bulk(value) => {
                    println!("{}", String::from_utf8_lossy(&value))
                }
                redis::protocol::Response::Nil => println!("(nil)"),
                redis::protocol::Response::Integer(value) => println!("{value}"),
                redis::protocol::Response::Error(error) => println!("{error}"),
            },
            Err(_) => {
                println!("ERR core unavailable");
                break;
            }
        }
    }
}
