use math::round;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Debug)]
pub enum RpcMessage {
    VoteRequest { term: u64, candidate_id: String },
    VoteResponse { term: u64, vote_granted: bool },
    Heartbeat { term: u64, peer_id: String },
    HeartbeatResponse { term: u64, peer_id: String },
}

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
    id: String,
    address: SocketAddrV4,
    state: State,
    term: u64,
    log_entries: Vec<LogEntry>,
    voted_for: Option<Peer>,
    next_timeout: Option<Instant>,
    config: ServerConfig,
    current_leader: Option<Leader>,
    number_of_peers: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: String,
}

pub struct VoteRequestResponse {
    pub term: u64,
    pub vote_granted: bool,
}

pub trait RpcClient {
    fn request_vote(&self, request: VoteRequest) -> Vec<VoteRequestResponse>;

    fn broadcast_log_entry(&self, log_entry: LogEntry);
}

pub trait RpcServer {
    fn start_server(&self);
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

    fn consume_log_entry(self: &mut Self, log_entry: &LogEntry) {
        match log_entry {
            LogEntry::Heartbeat { term, peer_id } => {
                self.refresh_timeout();

                if term > &self.term {
                    println!(
                        "Server {} becoming follower. The new leader is: {}",
                        self.id, peer_id
                    );

                    self.term = *term;
                    self.state = State::FOLLOWER;
                    self.voted_for = None;
                    self.current_leader = Some(Leader {
                        id: peer_id.to_string(),
                        term: *term,
                    })
                }
            }
        }
    }

    fn handle_vote_request(self: &mut Self, vote_request: VoteRequest) -> VoteRequestResponse {
        match self.voted_for {
            Some(_) => VoteRequestResponse {
                term: vote_request.term,
                vote_granted: false,
            },
            None => {
                if vote_request.term > self.term {
                    self.voted_for = Some(Peer {
                        id: vote_request.candidate_id,
                        address: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7879),
                    });

                    VoteRequestResponse {
                        term: vote_request.term,
                        vote_granted: true,
                    }
                } else {
                    VoteRequestResponse {
                        term: vote_request.term,
                        vote_granted: false,
                    }
                }
            }
        }
    }

    fn refresh_timeout(self: &mut Self) {
        println!("Server {} is refreshing its timeout.", self.id);
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
            id: self.id.to_string(),
            address: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7879),
        });

        Some(VoteRequest {
            term: self.term,
            candidate_id: self.id.to_string(),
        })
    }

    fn become_leader(self: &mut Self) {
        if self.state == State::CANDIDATE {
            println!(
                "Server {} has won the election! The new term is: {}",
                self.id, self.term
            );
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

pub fn handle_vote_request(
    server: Arc<Mutex<Server>>,
    request: VoteRequest,
) -> VoteRequestResponse {
    let server_clone = Arc::clone(&server);

    let vote_response = server_clone.lock().unwrap().handle_vote_request(request);

    vote_response
}

pub fn handle_heartbeat(server: Arc<Mutex<Server>>, entry: LogEntry) -> u64 {
    let server_clone = Arc::clone(&server);

    let server_id;
    let server_term;

    {
        let temp_server = server_clone.lock().unwrap();
        server_id = temp_server.id.to_string();
        server_term = temp_server.term;
    }

    if let LogEntry::Heartbeat { term, peer_id } = entry {
        println!(
            "Server {} with term {}, received heartbeat from {} with term {}",
            server_id, server_term, peer_id, term
        );
        server_clone
            .lock()
            .unwrap()
            .consume_log_entry(&LogEntry::Heartbeat {
                term: term,
                peer_id: peer_id,
            });
    };

    let current_term = server_clone.lock().unwrap().term;

    current_term
}

pub fn start_server(
    server: Arc<Mutex<Server>>,
    rpc_client: impl RpcClient + std::marker::Send + 'static,
) {
    let server_clone = Arc::clone(&server);

    server.lock().unwrap().start();

    let background_task_handle = thread::spawn(move || {
        background_task(server_clone, &rpc_client);
    });

    background_task_handle.join().unwrap();
}

fn background_task(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    loop {
        let clone = Arc::clone(&server);
        handle_timeout(clone, rpc_client);

        let clone = Arc::clone(&server);
        broadcast_heartbeat(clone, rpc_client);
    }
}

fn broadcast_heartbeat(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    let clone = Arc::clone(&server);
    let is_leader = clone.lock().unwrap().state == State::LEADER;

    if is_leader {
        let term = clone.lock().unwrap().term;
        let id = clone.lock().unwrap().id.to_string();

        rpc_client.broadcast_log_entry(LogEntry::Heartbeat {
            term: term,
            peer_id: id,
        });
    }

    thread::sleep(Duration::new(1, 0));
}

fn handle_timeout(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    let server_clone = Arc::clone(&server);
    let server_id = server_clone.lock().unwrap().id.to_string();

    if server_clone.lock().unwrap().has_timed_out() {
        println!("Server {} has timed out.", server_id);

        start_election(Arc::clone(&server_clone), rpc_client);
    }
}

fn start_election(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    let rpc_response: Option<Vec<VoteRequestResponse>>;
    let rpc_request = server.lock().unwrap().start_election();
    let server_id = server.lock().unwrap().id.to_string();
    let server_term = server.lock().unwrap().term;

    println!(
        "Server {}, with term {}, started the election process.",
        server_id, server_term
    );

    {
        rpc_response = match rpc_request {
            Some(r) => Some(rpc_client.request_vote(r)),
            None => None,
        };
    }

    if let Some(r) = rpc_response {
        let own_election;
        {
            let mut server = server.lock().unwrap();

            own_election = has_won_the_election(&server, r) && !server.has_timed_out();
        }

        if own_election {
            let clone = Arc::clone(&server);
            let mut server = clone.lock().unwrap();
            server.become_leader();
            let log_entry = LogEntry::Heartbeat {
                term: server.term,
                peer_id: server.id.to_string(),
            };

            rpc_client.broadcast_log_entry(log_entry);
        }
    }
}

fn has_won_the_election(server: &Server, response: Vec<VoteRequestResponse>) -> bool {
    let number_of_servers = server.number_of_peers + 1; // All peers + current server

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
        assert_eq!(server.number_of_peers, 0);
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
    fn handle_vote_request() {
        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(0, 0),
        });

        server.start();

        let candidate_id = String::new_v4();

        let vote_request = VoteRequest {
            candidate_id: candidate_id,
            term: server.term + 1,
        };

        let vote_response = server.handle_vote_request(vote_request);

        assert_eq!(vote_response.term, server.term + 1);
        assert!(vote_response.vote_granted);
        assert_eq!(server.voted_for.as_ref().unwrap().id, candidate_id);

        // Now the server has already voted for that term

        let candidate_id = String::new_v4();

        let vote_request = VoteRequest {
            candidate_id: candidate_id,
            term: server.term + 1,
        };

        let vote_response = server.handle_vote_request(vote_request);

        assert_eq!(vote_response.term, server.term + 1);
        assert!(!vote_response.vote_granted);
        assert_ne!(server.voted_for.as_ref().unwrap().id, candidate_id);

        // When the server did not vote yet, but the candidate's term is the same
        // as the current server.
        server.voted_for = None;

        let candidate_id = String::new_v4();

        let vote_request = VoteRequest {
            candidate_id: candidate_id,
            term: server.term,
        };

        let vote_response = server.handle_vote_request(vote_request);

        assert_eq!(vote_response.term, server.term);
        assert!(!vote_response.vote_granted);
        assert!(server.voted_for.as_ref().is_none());
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
            peer_id: String::new_v4(),
        };

        server.consume_log_entry(&log_entry);

        assert_eq!(server.state, State::FOLLOWER);
        assert_eq!(server.term, new_leader_current_term);
    }

    #[test]
    fn start_election_it() {
        let peers = create_peers(3);

        let fake_rpc = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(0, 0),
            peers: peers,
        };

        let mut server = Server::new(ServerConfig {
            timeout: Duration::new(2, 0),
        });

        server.start();
        server.number_of_peers = 3;

        let server = Arc::new(Mutex::new(server));

        start_election(Arc::clone(&server), &fake_rpc);

        assert_eq!(server.lock().unwrap().state, State::LEADER);
    }

    fn create_peers(n: usize) -> Vec<Peer> {
        let mut peers = Vec::new();

        for _ in 0..n {
            peers.push(Peer {
                id: String::new_v4(),
            });
        }

        peers
    }

    struct FakeRpc {
        granted_vote: bool,
        sleeps_for: Duration,
        peers: Vec<Peer>,
    }

    impl RpcClient for FakeRpc {
        fn request_vote(&self, request: VoteRequest) -> Vec<VoteRequestResponse> {
            let mut response = Vec::new();

            for _peer in self.peers.iter() {
                response.push(VoteRequestResponse {
                    term: request.term,
                    vote_granted: self.granted_vote,
                });
            }
            sleep(self.sleeps_for);
            response
        }

        fn broadcast_log_entry(&self, _log_entry: &LogEntry) {
            println!("broadcast");
        }
    }
}