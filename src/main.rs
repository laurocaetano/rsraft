mod raft;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

use raft::core;

fn main() {
    let server_config = core::ServerConfig {
        timeout: Duration::new(5, 0),
    };

    core::start_server(
        server_config,
        FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(10, 0),
        },
        create_peers(3),
    );
}

fn create_peers(n: usize) -> Vec<core::Peer> {
    let mut peers = Vec::new();

    for _ in 0..n {
        peers.push(core::Peer {
            id: Uuid::new_v4(),
            term: 0,
            state: core::State::FOLLOWER,
        });
    }

    peers
}
struct FakeRpc {
    granted_vote: bool,
    sleeps_for: Duration,
}

impl core::RpcServer for FakeRpc {
    fn broadcast_request_vote_rpc(
        &self,
        peers: &Vec<core::Peer>,
        _request: core::VoteRequest,
    ) -> Vec<core::VoteRequestResponse> {
        let mut response = Vec::new();

        for peer in peers.iter() {
            response.push(core::VoteRequestResponse {
                term: peer.term,
                vote_granted: self.granted_vote,
            });
        }
        sleep(self.sleeps_for);
        response
    }

    fn broadcast_log_entry_rpc(&self, _peers: &Vec<core::Peer>, _log_entry: &core::LogEntry) {
        println!("broadcast");
    }
}
