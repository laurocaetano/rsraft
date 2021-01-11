use math::round;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

enum LogEntry {
    HEARTBEAT,
}

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

        if (votes + 1) > min_quorum as usize {
            self.state = State::LEADER;
            rpc_server.broadcast_log_entry_rpc(&self.peers, &LogEntry::HEARTBEAT);
        }

        self.voted_for = None;
    }

    fn add_peer(self: &mut Self, peer: Peer) {
        self.peers.push(peer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn start_election() {
        let mut server = Server::new();
        let successful_rpcs = FakeRpc { granted_vote: true };
        let not_successful_rpcs = FakeRpc {
            granted_vote: false,
        };

        create_peers(3, &mut server);
        server.start_election(&not_successful_rpcs);

        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.term, 1);

        server.start_election(&not_successful_rpcs);
        assert_eq!(server.state, State::CANDIDATE);
        assert_eq!(server.term, 2);

        server.start_election(&successful_rpcs);
        assert_eq!(server.state, State::LEADER);
        assert_eq!(server.term, 3);

        // Starting a new election as a leader should not trigger election
        server.start_election(&successful_rpcs);
        assert_eq!(server.state, State::LEADER);
        assert_eq!(server.term, 3);
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

            response
        }

        fn broadcast_log_entry_rpc(&self, _peers: &Vec<Peer>, _log_entry: &LogEntry) {
            println!("broadcast");
        }
    }
}
