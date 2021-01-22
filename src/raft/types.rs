use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LogEntry {
    Heartbeat { term: u64, peer_id: String },
}

#[derive(Debug)]
pub struct Peer {
    pub id: String,
    pub address: SocketAddrV4,
}

#[derive(Debug)]
pub struct Leader {
    pub id: String,
    pub term: u64,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub timeout: Duration,
}

#[derive(Debug)]
pub struct Server {
    pub id: String,
    pub address: SocketAddrV4,
    pub state: State,
    pub term: u64,
    pub log_entries: Vec<LogEntry>,
    pub voted_for: Option<Peer>,
    pub next_timeout: Option<Instant>,
    pub config: ServerConfig,
    pub current_leader: Option<Leader>,
    pub number_of_peers: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: String,
}

pub struct VoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

pub trait RpcClient {
    fn request_vote(&self, request: VoteRequest) -> Vec<VoteResponse>;

    fn broadcast_log_entry(&self, log_entry: LogEntry);
}

impl Server {
    pub fn new(
        config: ServerConfig,
        number_of_peers: usize,
        address: SocketAddrV4,
        id: String,
    ) -> Self {
        Server {
            id: id,
            state: State::FOLLOWER,
            term: 0,
            log_entries: Vec::new(),
            voted_for: None,
            next_timeout: None,
            config: config,
            current_leader: None,
            number_of_peers: number_of_peers,
            address: address,
        }
    }

    pub fn refresh_timeout(self: &mut Self) {
        self.next_timeout = Some(Instant::now() + self.config.timeout);
    }

    pub fn become_leader(self: &mut Self) {
        if self.state == State::CANDIDATE {
            println!(
                "Server {} has won the election! The new term is: {}",
                self.id, self.term
            );
            self.state = State::LEADER;
            self.next_timeout = None;
        }
    }

    pub fn start(self: &mut Self) {
        self.refresh_timeout();
    }

    pub fn has_timed_out(self: &mut Self) -> bool {
        match self.next_timeout {
            Some(t) => Instant::now() > t,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn server_become_leader() {
        let mut server = build_server();

        server.become_leader();

        assert_eq!(server.state, State::FOLLOWER);

        // can only become leader if the current state
        // is "CANDIDATE".
        server.state = State::CANDIDATE;
        server.become_leader();

        assert_eq!(server.state, State::LEADER);
    }

    #[test]
    fn server_new() {
        let server = build_server();

        assert_eq!(server.state, State::FOLLOWER);
        assert_eq!(server.term, 0);
        assert_eq!(server.log_entries.len(), 0);
        assert!(server.voted_for.is_none());
        assert!(server.next_timeout.is_none());
        assert!(server.current_leader.is_none());
    }

    #[test]
    fn server_start() {
        let mut server = build_server();

        server.start();

        assert!(server.next_timeout.as_ref().unwrap() > &Instant::now());
    }

    #[test]
    fn server_has_timed_out() {
        let mut server = build_server();

        server.start();

        assert!(!server.has_timed_out());

        thread::sleep(Duration::new(1, 0));

        assert!(server.has_timed_out());
    }

    #[test]
    fn server_refresh_timeout() {
        let mut server = build_server();

        server.refresh_timeout();

        assert!(server.next_timeout.as_ref().unwrap() > &Instant::now());
    }

    fn build_server() -> Server {
        let config = ServerConfig {
            timeout: Duration::new(1, 0),
        };

        let number_of_peers = 2;
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9090);
        let id = "server_1".to_string();

        Server::new(config, number_of_peers, address, id)
    }
}
