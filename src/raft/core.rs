use math::round;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

#[derive(Debug)]
pub enum LogEntry {
    Heartbeat { term: u64, peer_id: Uuid },
}

#[derive(Debug)]
pub struct Peer {
    pub id: Uuid,
    pub term: u64,
    pub state: State,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub timeout: Duration,
}

#[derive(Debug)]
struct Server {
    id: Uuid,
    state: State,
    term: u64,
    peers: Vec<Peer>,
    log_entries: Vec<LogEntry>,
    voted_for: Option<Peer>,
    next_timeout: Option<Instant>,
    config: ServerConfig,
}

pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: Uuid,
}

pub struct VoteRequestResponse {
    pub term: u64,
    pub vote_granted: bool,
}

pub trait RpcServer {
    fn broadcast_request_vote_rpc(
        &self,
        peers: &Vec<Peer>,
        request: VoteRequest,
    ) -> Vec<VoteRequestResponse>;

    fn broadcast_log_entry_rpc(&self, peers: &Vec<Peer>, log_entry: &LogEntry);
}

impl Server {
    fn new(config: ServerConfig) -> Self {
        Server {
            id: Uuid::new_v4(),
            state: State::FOLLOWER,
            term: 0,
            peers: Vec::new(),
            log_entries: Vec::new(),
            voted_for: None,
            next_timeout: None,
            config: config,
        }
    }

    fn consume_log_entry(self: &mut Self, log_entry: &LogEntry) {
        match log_entry {
            LogEntry::Heartbeat { term, peer_id: _ } => {
                self.next_timeout = Some(Instant::now() + self.config.timeout);

                if term > &self.term {
                    self.term = *term;
                    self.state = State::FOLLOWER;
                }
            }
        }
    }

    fn refresh_timeout(self: &mut Self) {
        self.next_timeout = Some(Instant::now() + self.config.timeout);
    }

    fn start_election(self: &mut Self) -> Option<VoteRequest> {
        if self.state == State::LEADER {
            return None;
        }

        self.state = State::CANDIDATE;
        self.term = self.term + 1;
        self.refresh_timeout();
        self.voted_for = Some(Peer {
            id: self.id,
            term: self.term,
            state: State::CANDIDATE,
        });

        Some(VoteRequest {
            term: self.term,
            candidate_id: self.id,
        })
    }

    fn become_leader(self: &mut Self) {
        if self.state == State::CANDIDATE {
            self.state = State::LEADER;
            self.next_timeout = None;
        }
    }

    fn start(self: &mut Self) {
        self.refresh_timeout();
    }

    fn has_timed_out(self: &mut Self) -> bool {
        match self.next_timeout {
            Some(t) => Instant::now() > t,
            None => false,
        }
    }
}

pub fn start_server(
    config: ServerConfig,
    rpc_server: impl RpcServer + std::marker::Send + 'static,
    peers: Vec<Peer>,
) {
    let server = Arc::new(Mutex::new(Server::new(config)));
    let server_clone = Arc::clone(&server);

    server.lock().unwrap().peers = peers;
    server.lock().unwrap().start();

    let timeout_handle = thread::spawn(|| {
        handle_timeout(server_clone, rpc_server);
    });

    let server_clone = Arc::clone(&server);
    let heartbeat_handle = thread::spawn(|| {
        listen_to_heartbeats(server_clone);
    });

    timeout_handle.join().unwrap();
    heartbeat_handle.join().unwrap();
}

fn listen_to_heartbeats(_server: Arc<Mutex<Server>>) {}

fn handle_timeout(server: Arc<Mutex<Server>>, rpc_server: impl RpcServer) {
    println!("Handling timeout");

    let server_clone = Arc::clone(&server);

    loop {
        if server_clone.lock().unwrap().has_timed_out() {
            println!("Timed out");
            start_election(&mut server_clone.lock().unwrap(), &rpc_server);
        }
    }
}

fn start_election(server: &mut Server, rpc_server: &impl RpcServer) {
    println!("Started Election");

    let rpc_response = match server.start_election() {
        Some(r) => Some(rpc_server.broadcast_request_vote_rpc(&server.peers, r)),
        None => None,
    };

    println!("Current state: {:#?}", server);

    if let Some(rpc_response) = rpc_response {
        if has_won_the_election(server, rpc_response) && !server.has_timed_out() {
            println!("Has won the election!");

            server.become_leader();
        }
    }
}

fn has_won_the_election(server: &Server, response: Vec<VoteRequestResponse>) -> bool {
    let number_of_servers = server.peers.len() + 1; // All peers + current server

    let votes = response.iter().filter(|r| r.vote_granted).count();

    let min_quorum = round::floor((number_of_servers / 2) as f64, 0);

    (votes + 1) > min_quorum as usize && State::CANDIDATE == server.state
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn new_server() {
        let server = Server::new(ServerConfig {
            timeout: Duration::new(0, 0),
        });

        assert_eq!(server.term, 0);
        assert_eq!(server.state, State::FOLLOWER);
        assert_eq!(server.peers.len(), 0);
        assert_eq!(server.log_entries.len(), 0);
        assert!(server.voted_for.is_none());
    }

    #[test]
    fn start_election_ut() {
        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(10, 0),
        });

        server.start();

        let vote_request = server.start_election().unwrap();
        assert_eq!(server.term, 1);
        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);
        assert!(server.next_timeout.is_some());
        assert_eq!(vote_request.term, 1);
        assert_eq!(vote_request.candidate_id, server.id);
    }

    #[test]
    fn become_leader() {
        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(10, 0),
        });

        // When the server has started the election and is
        // a candidate already.
        server.start();
        server.start_election();
        server.become_leader();

        assert_eq!(server.state, State::LEADER);
        assert!(server.next_timeout.is_none());

        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(10, 0),
        });

        // When the server has started the election and is
        // but another server has been elected as leader.
        server.start();
        server.become_leader();

        assert_eq!(server.state, State::FOLLOWER);
        assert!(server.next_timeout.is_some());
    }

    #[test]
    fn consume_log_entry() {
        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(0, 0),
        });

        // Simulate that the server is the current leader
        server.state = State::LEADER;

        // Simulate that a new leader was elected
        let new_leader_current_term = 44;
        let log_entry = LogEntry::Heartbeat {
            term: new_leader_current_term,
            peer_id: Uuid::new_v4(),
        };

        server.consume_log_entry(&log_entry);

        assert_eq!(server.state, State::FOLLOWER);
        assert_eq!(server.term, new_leader_current_term);
    }

    #[test]
    fn start_election_it() {
        let fake_rpc = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(0, 0),
        };

        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(2, 0),
        });

        server.peers = create_peers(3);
        server.start();

        start_election(&mut server, &fake_rpc);

        assert_eq!(server.state, State::LEADER);
    }

    fn create_peers(n: usize) -> Vec<Peer> {
        let mut peers = Vec::new();

        for _ in 0..n {
            peers.push(Peer {
                id: Uuid::new_v4(),
                term: 0,
                state: State::FOLLOWER,
            });
        }

        peers
    }
    struct FakeRpc {
        granted_vote: bool,
        sleeps_for: Duration,
    }

    impl RpcServer for FakeRpc {
        fn broadcast_request_vote_rpc(
            &self,
            peers: &Vec<Peer>,
            _request: VoteRequest,
        ) -> Vec<VoteRequestResponse> {
            let mut response = Vec::new();

            for peer in peers.iter() {
                response.push(VoteRequestResponse {
                    term: peer.term,
                    vote_granted: self.granted_vote,
                });
            }
            sleep(self.sleeps_for);
            response
        }

        fn broadcast_log_entry_rpc(&self, _peers: &Vec<Peer>, _log_entry: &LogEntry) {
            println!("broadcast");
        }
    }
}
