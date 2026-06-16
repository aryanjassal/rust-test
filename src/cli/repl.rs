use std::io::{self, Write};

use crate::{
    cli::parser,
    client::tcp::TcpClient,
    redis::{parser::parse_command, protocol::Response},
};

pub fn run(addr: &str) -> Result<(), String> {
    let mut client = TcpClient::connect(addr).map_err(|e| e.to_string())?;
    let mut buffer = String::new();

    loop {
        buffer.clear();

        print!("\n> ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let read = io::stdin()
            .read_line(&mut buffer)
            .map_err(|e| e.to_string())?;

        if read == 0 {
            break;
        }

        let input = buffer.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        let parts = parser::split(input)
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let command = match parse_command(&parts.iter().map(|v| v.as_bytes().to_vec()).collect()) {
            Ok(command) => command,
            Err(error) => {
                println!("{error}");
                continue;
            }
        };

        match client.send(command) {
            Ok(response) => print_response(response),
            Err(error) => {
                println!("ERR {error}");
                break;
            }
        }
    }

    Ok(())
}

fn print_response(response: Response) {
    match response {
        Response::Simple(value) => println!("{value}"),
        Response::Bulk(value) => println!("{}", String::from_utf8_lossy(&value)),
        Response::Nil => println!("(nil)"),
        Response::Integer(value) => println!("{value}"),
        Response::Error(error) => println!("{error}"),
    }
}
