mod raft;

use raft::core;
use rand::{thread_rng, Rng};
use std::sync::mpsc::channel;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

fn main() {
    let server_config = core::ServerConfig {
        timeout: Duration::new(4, 0),
    };

    let (send, recv) = channel();

    let fake_heartbeat_sender = thread::spawn(move || {
        let mut term = 1;

        loop {
            send.send(core::LogEntry::Heartbeat {
                term: term,
                peer_id: Uuid::new_v4(),
            })
            .unwrap();

            thread::sleep(Duration::new(thread_rng().gen_range(1..10), 0));

            term = term + 1;
        }
    });

    let peers = create_peers(3);
    let number_of_peers = peers.len();

    core::start_server(
        server_config,
        FakeRpc {
            granted_vote: true,
            sleeps_for: Duration::new(10, 0),
            peers: peers,
        },
        recv,
        number_of_peers,
    );

    fake_heartbeat_sender.join().unwrap();
}

fn create_peers(n: usize) -> Vec<core::Peer> {
    let mut peers = Vec::new();

    for _ in 0..n {
        peers.push(core::Peer { id: Uuid::new_v4() });
    }

    peers
}

struct FakeRpc {
    granted_vote: bool,
    sleeps_for: Duration,
    peers: Vec<core::Peer>,
}

impl core::RpcServer for FakeRpc {
    fn broadcast_request_vote_rpc(
        &self,
        request: core::VoteRequest,
    ) -> Vec<core::VoteRequestResponse> {
        let mut response = Vec::new();

        for _peer in self.peers.iter() {
            response.push(core::VoteRequestResponse {
                term: request.term,
                vote_granted: self.granted_vote,
            });
        }
        sleep(self.sleeps_for);
        response
    }

    fn broadcast_log_entry_rpc(&self, _log_entry: &core::LogEntry) {
        println!("broadcast");
    }
}
