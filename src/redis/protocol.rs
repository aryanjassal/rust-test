use std::sync::mpsc::Sender;

/// The key used for indexing can only be arbitrary bytes
pub type RedisKey = Vec<u8>;

/// The value can be multiple specialised types. Currently, only the following
/// types are supported:
/// - Bulk string
pub enum RedisValue {
    Str(Vec<u8>),
}

/// Each command either returns a success value or a failure message.
/// TODO: figure out the best way to indicate failure
pub type RedisResult<T> = Result<T, String>;

pub enum Command {
    Set { key: RedisKey, value: RedisValue },
    Get { key: RedisKey },
    Del { keys: Vec<RedisKey> },
    Save,
}

pub enum Response {
    Ok,
    Nil,
    Bulk(Vec<u8>),
    Integer(i64),
    Error(String),
}

pub struct Request {
    pub command: Command,
    pub reply: Sender<Response>,
}
