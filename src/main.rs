mod raft;
use raft::core;
use std::time::Duration;

fn main() {
    let config = core::ServerConfig {
        timeout: Duration::new(1, 0),
    };

    let server = core::start_server(config);
}
