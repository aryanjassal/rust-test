use std::io::Write;

use crate::redis::{
    parser::generate_error,
    protocol::{Command, RedisResult, RedisValue, Response},
};

fn encode_bulk_string(value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("${}\r\n", value.len()).as_bytes());
    out.extend_from_slice(value);
    out.extend_from_slice(b"\r\n");
    out
}

fn encode_array_size(len: usize) -> Vec<u8> {
    format!("*{len}\r\n").as_bytes().to_vec()
}

pub fn write_command<W: Write>(writer: &mut W, command: Command) -> std::io::Result<()> {
    match command {
        Command::Set { key, value } => {
            writer.write_all(&encode_array_size(3))?;
            writer.write_all(&encode_bulk_string("SET".as_bytes()))?;
            writer.write_all(&encode_bulk_string(&key))?;
            match value {
                RedisValue::Str(val) => writer.write_all(&encode_bulk_string(&val)),
            }
        }
        Command::Get { key } => {
            writer.write_all(&encode_array_size(2))?;
            writer.write_all(&encode_bulk_string("GET".as_bytes()))?;
            writer.write_all(&encode_bulk_string(&key))
        }
        Command::Del { keys } => {
            writer.write_all(&encode_array_size(1 + keys.len()))?;
            writer.write_all(&encode_bulk_string("DEL".as_bytes()))?;
            for key in keys {
                writer.write_all(&encode_bulk_string(&key))?;
            }
            Ok(())
        }
        Command::Save => {
            writer.write_all(&encode_array_size(1))?;
            writer.write_all(&encode_bulk_string("SAVE".as_bytes()))
        }
    }
}

pub fn write_response<W: Write>(writer: &mut W, response: Response) -> std::io::Result<()> {
    match response {
        Response::Simple(value) => write!(writer, "+{value}\r\n"),
        Response::Bulk(value) => {
            write!(writer, "${}\r\n", value.len())?;
            writer.write_all(&value)?;
            writer.write_all(b"\r\n")
        }
        Response::Nil => writer.write_all(b"$-1\r\n"),
        Response::Integer(value) => write!(writer, ":{value}\r\n"),
        Response::Error(error) => write!(writer, "-{error}\r\n"),
    }
}

pub fn read_command<R: std::io::BufRead>(reader: &mut R) -> Result<Vec<Vec<u8>>, String> {
    // Get length of RESP array
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;

    if !line.starts_with('*') {
        return Err(generate_error("expected RESP array".to_string()));
    }

    let count = parse_len(&line[1..])?;
    let mut parts = Vec::with_capacity(count);

    // Split the input to parts of the array
    for _ in 0..count {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(_) if line.starts_with('$') => {
                let len = parse_len(&line[1..])?;
                let mut buf = vec![0u8; len + 2];
                reader.read_exact(&mut buf).map_err(|e| e.to_string())?;

                if &buf[len..] != b"\r\n" {
                    return Err(generate_error("invalid bulk terminator".to_string()));
                }
                parts.push(buf[..len].to_vec())
            }
            Ok(_) => {
                return Err(generate_error("invalid command".to_string()));
            }
            Err(error) => return Err(generate_error(error.to_string())),
        };
    }

    Ok(parts)
}

fn read_crlf_line<R: std::io::BufRead>(reader: &mut R) -> RedisResult<Vec<u8>> {
    let mut buf = Vec::new();
    reader
        .read_until(b'\n', &mut buf)
        .map_err(|e| e.to_string())?;
    if buf.len() < 2 || &buf[buf.len() - 2..] != b"\r\n" {
        return Err("invalid CRLF terminator".into());
    }
    buf.truncate(buf.len() - 2);
    Ok(buf)
}

pub fn read_response<R: std::io::BufRead>(reader: &mut R) -> RedisResult<Response> {
    let mut prefix = [0u8; 1];
    reader.read_exact(&mut prefix).map_err(|e| e.to_string())?;

    match prefix[0] {
        b'+' => {
            let bytes = read_crlf_line(reader)?;
            let text =
                String::from_utf8(bytes).map_err(|_| "invalid utf8 simple string".to_string())?;
            Ok(Response::Simple(text))
        }
        b'-' => {
            let bytes = read_crlf_line(reader)?;
            let text = String::from_utf8(bytes).map_err(|_| "invalid utf8 error".to_string())?;
            Ok(Response::Error(text))
        }
        b':' => {
            let bytes = read_crlf_line(reader)?;
            let text = String::from_utf8(bytes).map_err(|_| "invalid integer".to_string())?;
            let value = text
                .parse::<i64>()
                .map_err(|_| "invalid integer".to_string())?;
            Ok(Response::Integer(value))
        }
        b'$' => {
            let bytes = read_crlf_line(reader)?;
            let len = String::from_utf8(bytes)
                .map_err(|_| "invalid bulk length".to_string())?
                .parse::<isize>()
                .map_err(|_| "invalid bulk length".to_string())?;
            if len == -1 {
                return Ok(Response::Nil);
            }
            if len < 0 {
                return Err("invalid bulk length".into());
            }

            let len = len as usize;
            let mut buf = vec![0u8; len + 2];
            reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
            if &buf[len..] != b"\r\n" {
                return Err("invalid bulk terminator".into());
            }
            buf.truncate(len);
            Ok(Response::Bulk(buf))
        }
        _ => Err("unknown response type".into()),
    }
}

fn parse_len(input: &str) -> Result<usize, String> {
    input
        .trim_end_matches("\r\n")
        .parse::<usize>()
        .map_err(|_| generate_error("invalid length".to_string()))
}
