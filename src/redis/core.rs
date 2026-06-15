use std::{collections::HashMap, fs::File, io::Write, os::unix::fs::FileExt, sync::mpsc::Receiver};

use crate::redis::protocol::{Command, RedisKey, RedisResult, RedisValue, Request, Response};

const DUMP_MAGIC: &str = "RSTREDIS";
const DUMP_V1: &str = "V1\0\0\0\0\0\0";

pub struct DbCore {
    data: HashMap<RedisKey, RedisValue>,
    dump_path: String,
}

impl DbCore {
    pub fn new(dump_path: String) -> Self {
        Self {
            data: HashMap::new(),
            dump_path,
        }
    }

    pub fn run(mut self, rx: Receiver<Request>) {
        for request in rx {
            let response = self.execute(request.command);
            let _ = request.reply.send(response);
        }
    }

    fn execute(&mut self, command: Command) -> Response {
        match command {
            Command::Set { key, value } => {
                self.data.insert(key, value);
                Response::Ok
            }
            Command::Get { key } => match self.data.get(&key) {
                Some(value) => match value {
                    RedisValue::Str(val) => Response::Bulk(val.clone()),
                },
                None => Response::Nil,
            },
            Command::Del { keys } => {
                let mut removed = 0;
                for key in keys {
                    if self.data.remove(&key).is_some() {
                        removed += 1;
                    }
                }
                Response::Integer(removed)
            }
            Command::Save => {
                self.save().unwrap();
                Response::Ok
            }
        }
    }

    fn save(&self) -> RedisResult<()> {
        let mut db = match File::create(self.dump_path.clone()) {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };

        // Write the file header to disk.
        // [MAGIC]: RSTREDIS
        // [VER]: V1\0\0\0\0\0\0
        // Total: 16 bytes

        db.write_all(DUMP_MAGIC.as_bytes()).unwrap();
        db.write_all(DUMP_V1.as_bytes()).unwrap();

        // Write all entries to disk.
        // Iterate over all entries and store them sequentially.
        // [FLAGS]: 0b00000111 (first three bytes for the type, rest unused)
        // [KEY_LEN]: u64
        // [VAL_LEN]: u64
        // [KEY]: dynamic length data
        // [VAL]: dynamic length data
        // Note: only 3 bits are used in the flags. 0 means string, while other
        // states are reserved for now.

        for (key, value) in self.data.iter() {
            match value {
                RedisValue::Str(val) => {
                    db.write_all(&(0b00000000u8).to_le_bytes()).unwrap();
                    db.write_all(&key.len().to_le_bytes()).unwrap();
                    db.write_all(&val.len().to_le_bytes()).unwrap();
                    db.write_all(key).unwrap();
                    db.write_all(val).unwrap();
                }
            }
        }

        // Write 0xff as a sentinel value
        db.write_all(&(255u8).to_le_bytes()).unwrap();

        // Ensure data is written to disk
        return match db.sync_data() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }

    pub fn load(&mut self) -> RedisResult<()> {
        // Try loading the dump. If the dump doesn't exist then exit early.
        let db = match File::open(self.dump_path.clone()) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        let mut cursor: u64 = 0;

        // Read magic
        let magic = read_array_at::<8>(&db, &mut cursor)?;
        if magic != DUMP_MAGIC.as_bytes() {
            return Err("Invalid magic".to_string());
        }

        // Read version
        let version = read_array_at::<8>(&db, &mut cursor)?;
        if version != DUMP_V1.as_bytes() {
            return Err("Invalid version".to_string());
        }

        loop {
            // Check for sentinel value
            let flags = read_array_at::<1>(&db, &mut cursor)?[0];
            if flags == 0xff {
                break;
            };

            // Flags currently do nothing, so they can be ignored.
            // TODO: account for flags when multiple types are introduced

            // Read key and value lengths
            let key_len = u64::from_le_bytes(read_array_at::<8>(&db, &mut cursor)?);
            let value_len = u64::from_le_bytes(read_array_at::<8>(&db, &mut cursor)?);

            // Read key and value
            let key = read_vec_at(&db, &mut cursor, key_len as usize)?;
            let value = read_vec_at(&db, &mut cursor, value_len as usize)?;

            // Insert into db
            self.data.insert(key, RedisValue::Str(value));
        }

        Ok(())
    }
}

fn read_array_at<const N: usize>(db: &std::fs::File, cursor: &mut u64) -> RedisResult<[u8; N]> {
    let mut buf = [0u8; N];
    let len = db.read_at(&mut buf, *cursor).map_err(|e| e.to_string())?;

    if len != N {
        return Err(format!("Invalid read length: expected {N}, got {len}"));
    }

    *cursor += N as u64;
    Ok(buf)
}

fn read_vec_at(db: &std::fs::File, cursor: &mut u64, len: usize) -> RedisResult<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let read = db.read_at(&mut buf, *cursor).map_err(|e| e.to_string())?;

    if read != len {
        return Err(format!(
            "Invalid read length: expected {}, got {}",
            len, read
        ));
    }

    *cursor += len as u64;
    Ok(buf)
}
