use std::net::Ipv4Addr;
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
    address: Ipv4Addr,
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
}
