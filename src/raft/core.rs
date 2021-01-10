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
    fn start_election(mut self: &mut Self, rpc_server: &impl RpcServer) {
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

        let votes = rpc_response.iter().filter(|r| r.vote_granted).count();

        if votes > (round::floor(self.peers.len() as f64, 2) as usize) {
            self.state = State::LEADER;
            rpc_server.broadcast_log_entry_rpc(&self.peers, &LogEntry::HEARTBEAT);
        }
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

    fn start_election() {
        // TODO: Implemeting test cases
        let server = Server::new();
    }
}
