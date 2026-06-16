mod cli;
mod client;
mod redis;
mod server;

use std::{env, sync::mpsc, thread, time::Duration};

const DEFAULT_IP: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "6969";
const DEFAULT_DUMP: &str = "dump.rdb";

fn main() {
    let args = env::args().collect::<Vec<_>>();

    let mode = args.get(1).map(String::as_str).unwrap_or("dev");
    let ip = args.get(2).map(String::as_str).unwrap_or(DEFAULT_IP);
    let port = args.get(3).map(String::as_str).unwrap_or(DEFAULT_PORT);
    let addr = format!("{ip}:{port}");

    let result = match mode {
        "server" => run_server(&addr),
        "cli" => cli::run(&addr),
        "dev" => run_dev(&addr),
        unknown => Err(format!(
            "unknown mode '{unknown}'. expected: server, cli, or no argument"
        )),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run_server(addr: &str) -> Result<(), String> {
    let (core_tx, core_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut core = server::core::DbCore::new(DEFAULT_DUMP.to_string());
        if let Err(error) = core.load() {
            eprintln!("failed to load dump: {error}");
        }
        core.run(core_rx);
    });

    server::tcp::run(addr, core_tx).map_err(|e| e.to_string())
}

fn run_dev(addr: &str) -> Result<(), String> {
    let addr_for_server = addr.to_string();
    thread::spawn(move || {
        if let Err(error) = run_server(&addr_for_server) {
            eprintln!("server failed: {error}");
        }
    });

    // Small delay so the listener has time to bind before the client connects.
    // TODO: Replace this with a readiness channel.
    thread::sleep(Duration::from_millis(50));

    cli::run(addr)
}
