use crate::raft::types::{LogEntry, Peer, RpcClient, Server, State, VoteRequest, VoteResponse};
use math::round;
use std::sync::Arc;
use std::sync::Mutex;

fn new_election(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    let vote_response: Option<Vec<VoteResponse>>;
    let vote_request = prepare_vote_request(Arc::clone(&server));

    let server_id = server.lock().unwrap().id.to_string();
    let server_current_term = server.lock().unwrap().term;

    println!(
        "Server {}, with term {}, started the election process.",
        server_id, server_current_term
    );

    {
        vote_response = match vote_request {
            Some(request) => Some(rpc_client.request_vote(request)),
            None => None,
        };
    }

    if let Some(r) = vote_response {
        let own_election;
        {
            let mut server = server.lock().unwrap();
            own_election = has_won_the_election(&server, r) && !server.has_timed_out();
        }

        if own_election {
            become_leader(Arc::clone(&server), rpc_client);
        }
    }
}

fn prepare_vote_request(server: Arc<Mutex<Server>>) -> Option<VoteRequest> {
    let server_clone = Arc::clone(&server);

    if server_clone.lock().unwrap().state == State::LEADER {
        return None;
    }

    {
        let mut server_tmp = server_clone.lock().unwrap();
        server_tmp.state = State::CANDIDATE;
        server_tmp.term = server_tmp.term + 1;
        server_tmp.refresh_timeout();
        server_tmp.voted_for = Some(Peer {
            id: server_tmp.id.to_string(),
            address: server_tmp.address,
        });
    }

    let new_term = server_clone.lock().unwrap().term;
    let id = server_clone.lock().unwrap().id.to_string();
    Some(VoteRequest {
        term: new_term,
        candidate_id: id,
    })
}

fn has_won_the_election(server: &Server, response: Vec<VoteResponse>) -> bool {
    let number_of_servers = server.number_of_peers + 1; // All peers + current server

    let votes = response.iter().filter(|r| r.vote_granted).count();

    let min_quorum = round::floor((number_of_servers / 2) as f64, 0);

    (votes + 1) > min_quorum as usize && State::CANDIDATE == server.state
}

fn become_leader(server: Arc<Mutex<Server>>, rpc_client: &impl RpcClient) {
    let clone = Arc::clone(&server);
    let mut server = clone.lock().unwrap();

    server.become_leader();

    let log_entry = LogEntry::Heartbeat {
        term: server.term,
        peer_id: server.id.to_string(),
    };

    rpc_client.broadcast_log_entry(log_entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::types::ServerConfig;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn raft_new_election() {
        // When the server gets the vote from its peers
        let server = Arc::new(Mutex::new(build_server()));
        let rpc_client = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(0, 0),
            peers: create_peers(2),
        };

        new_election(Arc::clone(&server), &rpc_client);

        {
            let tmp_server = server.lock().unwrap();
            assert_eq!(tmp_server.state, State::LEADER);
            assert_eq!(tmp_server.term, 1);
        }

        // When the server does not get the vote from its peers
        let server = Arc::new(Mutex::new(build_server()));
        let rpc_client = FakeRpc {
            granted_vote: false,
            sleeps_for: Duration::new(0, 0),
            peers: create_peers(2),
        };

        new_election(Arc::clone(&server), &rpc_client);

        {
            let tmp_server = server.lock().unwrap();
            assert_eq!(tmp_server.state, State::CANDIDATE);
            assert_eq!(tmp_server.term, 1);
        }

        // When the server is alredy leader.
        let server = Arc::new(Mutex::new(build_server()));
        let rpc_client = FakeRpc {
            granted_vote: false,
            sleeps_for: Duration::new(0, 0),
            peers: create_peers(2),
        };

        server.lock().unwrap().state = State::LEADER;
        server.lock().unwrap().term = 10;

        new_election(Arc::clone(&server), &rpc_client);

        {
            let tmp_server = server.lock().unwrap();
            assert_eq!(tmp_server.state, State::LEADER);
            // term does not change
            assert_eq!(tmp_server.term, 10);
        }

        // When the server times out again, it should not
        // become leader even when getting votes.
        let server = Arc::new(Mutex::new(build_server()));
        let rpc_client = FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(1, 0),
            peers: create_peers(2),
        };

        server.lock().unwrap().start();

        new_election(Arc::clone(&server), &rpc_client);

        {
            let tmp_server = server.lock().unwrap();
            assert_eq!(tmp_server.state, State::CANDIDATE);
            assert_eq!(tmp_server.term, 1);
        }
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

    fn create_peers(n: usize) -> Vec<Peer> {
        let mut peers = Vec::new();

        for i in 0..n {
            peers.push(Peer {
                id: i.to_string(),
                address: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9090),
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
        fn request_vote(&self, request: VoteRequest) -> Vec<VoteResponse> {
            let mut response = Vec::new();

            for _peer in self.peers.iter() {
                response.push(VoteResponse {
                    term: request.term,
                    vote_granted: self.granted_vote,
                });
            }
            sleep(self.sleeps_for);
            response
        }

        fn broadcast_log_entry(&self, _log_entry: LogEntry) {
            println!("broadcast");
        }
    }
}
