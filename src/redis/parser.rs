use crate::redis::protocol::{Command, RedisResult, RedisValue};

fn generate_error(body: String) -> String {
    return format!("ERR {}", body);
}

fn generate_arity_error(command: String) -> String {
    return generate_error(format!("wrong number of arguments for {}", command));
}

pub fn parse_command(parts: &[String]) -> RedisResult<Command> {
    if parts.is_empty() {
        return Err("empty command".to_string());
    }

    match parts[0].to_ascii_uppercase().as_str() {
        "SET" => {
            if parts.len() != 3 {
                return Err(generate_arity_error("SET".to_string()));
            }
            Ok(Command::Set {
                key: parts[1].as_bytes().to_vec(),
                value: RedisValue::Str(parts[2].as_bytes().to_vec()),
            })
        }
        "GET" => {
            if parts.len() != 2 {
                return Err(generate_arity_error("GET".to_string()));
            }
            Ok(Command::Get {
                key: parts[1].as_bytes().to_vec(),
            })
        }
        "DEL" => {
            if parts.len() < 2 {
                return Err(generate_arity_error("DEL".to_string()));
            }
            Ok(Command::Del {
                keys: parts[1..].iter().map(|s| s.as_bytes().to_vec()).collect(),
            })
        }
        "SAVE" => {
            if parts.len() != 1 {
                return Err(generate_arity_error("SAVE".to_string()));
            }
            Ok(Command::Save)
        }
        name => Err(generate_error(format!("unknown command '{}'", name))),
    }
}
