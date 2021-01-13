use math::round;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

pub enum LogEntry {
    Heartbeat { term: u64, peer_id: Uuid },
}

pub struct Peer {
    pub id: Uuid,
    pub term: u64,
    pub state: State,
}

pub struct ServerConfig {
    pub timeout: Duration,
}

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

pub fn start_server<T: RpcServer + std::marker::Sync>(config: ServerConfig) {
    let mut server = Server::new(config);
    server.start();
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn start_election() {
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
}
