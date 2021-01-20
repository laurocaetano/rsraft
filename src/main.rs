mod raft;

use raft::core;
use raft::core::{RpcClient, RpcServer};
use raft::tcp_rpc_client::TcpRpcClient;
use raft::tcp_rpc_server::TcpRpcServer;
use rand::{thread_rng, Rng};
use std::convert::TryInto;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::mpsc::channel;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

fn main() {
    let server_config = core::ServerConfig {
        timeout: Duration::new(3, 0),
    };

    let (send, recv) = channel();

    let number_of_peers = 3;
    let peers = create_peers(number_of_peers);

    let mut servers = Vec::new();
    for i in 0..number_of_peers {
        servers.push(thread::spawn(move || {
            let tcp_rpc_server = TcpRpcServer;
            let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, (3300 + i).try_into().unwrap());
            tcp_rpc_server.start_server(address);
        }));
    }

    let tcp_rpc_client = TcpRpcClient::new(&peers);

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

    core::start_server(server_config, tcp_rpc_client, recv, number_of_peers);

    fake_heartbeat_sender.join().unwrap();

    for rpc_child in servers {
        rpc_child.join().unwrap();
    }
}

fn create_peers(n: usize) -> Vec<core::Peer> {
    let mut peers = Vec::new();

    for i in 0..n {
        peers.push(core::Peer {
            id: Uuid::new_v4(),
            address: SocketAddrV4::new(Ipv4Addr::LOCALHOST, (3300 + i).try_into().unwrap()),
        });
    }

    peers
}

struct FakeRpc {
    granted_vote: bool,
    sleeps_for: Duration,
    peers: Vec<core::Peer>,
}

impl core::RpcClient for FakeRpc {
    fn request_vote(&self, request: core::VoteRequest) -> Vec<core::VoteRequestResponse> {
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

    fn broadcast_log_entry(&self, _log_entry: core::LogEntry) {
        println!("broadcast");
    }
}
