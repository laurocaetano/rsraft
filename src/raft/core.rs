use math::round;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, PartialEq)]
enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

enum LogEntry {
    Heartbeat { term: u64, peer_id: Uuid },
}

#[derive(Debug, PartialEq)]
struct Peer {
    id: Uuid,
    term: u64,
    state: State,
}

struct Server {
    id: Uuid,
    state: State,
    term: u64,
    peers: Vec<Peer>,
    log_entries: Vec<LogEntry>,
    voted_for: Option<Peer>,
    next_timeout: Instant,
    timeout_config: Duration,
}

struct RequestVoteRequestRpc {
    term: u64,
    candidate_id: Uuid,
}

struct RequestVoteResponseRpc {
    term: u64,
    vote_granted: bool,
}

trait RpcServer {
    fn broadcast_request_vote_rpc(
        &self,
        peers: &Vec<Peer>,
        request: RequestVoteRequestRpc,
    ) -> Vec<RequestVoteResponseRpc>;

    fn broadcast_log_entry_rpc(&self, peers: &Vec<Peer>, log_entry: &LogEntry);
}

impl Server {
    fn new() -> Self {
        Server {
            id: Uuid::new_v4(),
            state: State::FOLLOWER,
            term: 0,
            peers: Vec::new(),
            log_entries: Vec::new(),
            voted_for: None,
            timeout_config: Duration::new(1, 0),
            next_timeout: Instant::now() + Duration::new(1, 0),
        }
    }

    fn consume_log_entry(self: &mut Self, log_entry: &LogEntry) {
        match log_entry {
            LogEntry::Heartbeat { term, peer_id } => {
                if term > &self.term {
                    self.term = *term;
                    self.state = State::FOLLOWER;
                }
            }
        }
    }

    // This is the core dump for the leader election algorithm, that is yet to be
    // refined. Please do not judge the quality of the code at this point in time :)
    fn start_election(self: &mut Self, rpc_server: &impl RpcServer) {
        if self.state == State::LEADER {
            return;
        }

        self.term = self.term + 1;
        self.state = State::CANDIDATE;
        self.voted_for = Some(Peer {
            id: self.id,
            term: self.term,
            state: State::CANDIDATE,
        });

        let request_vote_rpc = RequestVoteRequestRpc {
            term: self.term,
            candidate_id: self.id,
        };

        let rpc_response = rpc_server.broadcast_request_vote_rpc(&self.peers, request_vote_rpc);

        let number_of_servers = self.peers.len() + 1; // All peers + current server

        let votes = rpc_response.iter().filter(|r| r.vote_granted).count();

        let min_quorum = round::floor((number_of_servers / 2) as f64, 0);

        // If election times out, abort the current one and starts a new one.
        // For now it just returns, but the timeout logic is still to be implemented.
        if Instant::now() > self.next_timeout {
            self.next_timeout = Instant::now() + self.timeout_config;
            return;
        }

        if (votes + 1) > min_quorum as usize {
            let max_nanoseconds = u64::MAX / 1_000_000_000;
            self.next_timeout = Instant::now() + Duration::new(max_nanoseconds, 0);
            self.state = State::LEADER;

            rpc_server.broadcast_log_entry_rpc(
                &self.peers,
                &LogEntry::Heartbeat {
                    term: self.term,
                    peer_id: self.id,
                },
            );
        }
    }

    fn add_peer(self: &mut Self, peer: Peer) {
        self.peers.push(peer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn new_server() {
        let server = Server::new();

        assert_eq!(server.term, 0);
        assert_eq!(server.state, State::FOLLOWER);
        assert_eq!(server.peers.len(), 0);
        assert_eq!(server.log_entries.len(), 0);
        assert!(server.voted_for.is_none());
    }

    #[test]
    fn consume_log_entry() {
        let mut server = Server::new();

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
    fn start_election() {
        let mut server = Server::new();
        let successful_rpcs = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(0, 0),
        };
        let not_successful_rpcs = FakeRpc {
            granted_vote: false,
            sleeps_for: Duration::new(0, 0),
        };

        let timed_out = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(1, 0),
        };

        create_peers(3, &mut server);
        server.start_election(&not_successful_rpcs);

        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.term, 1);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);

        server.start_election(&not_successful_rpcs);
        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.term, 2);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);

        server.start_election(&timed_out);
        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.term, 3);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);

        server.start_election(&successful_rpcs);
        assert_eq!(server.state, State::LEADER);
        assert_eq!(server.term, 4);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);

        // Starting a new election as a leader should not trigger election
        server.start_election(&successful_rpcs);
        assert_eq!(server.state, State::LEADER);
        assert_eq!(server.term, 4);
        assert_eq!(server.voted_for.as_ref().unwrap().id, server.id);
    }

    fn create_peers(n: u32, server: &mut Server) {
        for _ in 0..n {
            server.add_peer(Peer {
                id: Uuid::new_v4(),
                term: 0,
                state: State::FOLLOWER,
            });
        }
    }

    struct FakeRpc {
        granted_vote: bool,
        sleeps_for: Duration,
    }

    impl RpcServer for FakeRpc {
        fn broadcast_request_vote_rpc(
            &self,
            peers: &Vec<Peer>,
            _request: RequestVoteRequestRpc,
        ) -> Vec<RequestVoteResponseRpc> {
            let mut response = Vec::new();

            for peer in peers.iter() {
                response.push(RequestVoteResponseRpc {
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
