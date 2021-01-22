use crate::raft::types::{LogEntry, Peer, RpcClient, Server, VoteRequest, VoteResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddrV4;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Serialize, Deserialize, Debug)]
enum RpcMessage {
    VoteRequest { term: u64, candidate_id: String },
    VoteResponse { term: u64, vote_granted: bool },
    Heartbeat { term: u64, peer_id: String },
    HeartbeatResponse { term: u64, peer_id: String },
}

pub struct TcpRpcClient {
    servers: HashMap<String, TcpStream>,
}

pub struct TcpRpcServer {
    server: Arc<Mutex<Server>>,
    address: SocketAddrV4,
}

impl RpcClient for TcpRpcClient {
    fn request_vote(&self, request: VoteRequest) -> Vec<VoteResponse> {
        let rpc_message = RpcMessage::VoteRequest {
            term: request.term,
            candidate_id: request.candidate_id,
        };

        let request_vote_bin = bincode::serialize(&rpc_message).unwrap();
        let mut rpc_responses: Vec<RpcMessage> = Vec::new();

        for mut stream in self.servers.values() {
            stream.write(&request_vote_bin).unwrap();

            let mut buffer = [0; 256];
            stream.read(&mut buffer).unwrap();
            rpc_responses.push(bincode::deserialize(&buffer).unwrap());
        }

        let mut response = Vec::new();
        for rpc_resp in rpc_responses {
            if let RpcMessage::VoteResponse { term, vote_granted } = rpc_resp {
                response.push(VoteResponse {
                    term: term,
                    vote_granted: vote_granted,
                });
            }
        }

        response
    }

    fn broadcast_log_entry(&self, log_entry: LogEntry) {
        if let LogEntry::Heartbeat { term, peer_id } = log_entry {
            let rpc_message = RpcMessage::Heartbeat {
                term: term,
                peer_id: peer_id,
            };

            let broadcast_bin = bincode::serialize(&rpc_message).unwrap();

            for mut stream in self.servers.values() {
                stream.write(&broadcast_bin).unwrap();

                let mut buffer = [0; 256];
                stream.read(&mut buffer).unwrap();
            }
        }
    }
}

impl TcpRpcClient {
    pub fn new(peers: &Vec<Peer>) -> Self {
        let mut servers = HashMap::new();

        for peer in peers.iter() {
            let stream = TcpStream::connect(&peer.address).unwrap();
            servers.insert(peer.id.to_string(), stream);
        }

        TcpRpcClient { servers: servers }
    }
}

impl TcpRpcServer {
    pub fn new(server: Arc<Mutex<Server>>, address: SocketAddrV4) -> Self {
        TcpRpcServer {
            server: Arc::clone(&server),
            address: address,
        }
    }

    pub fn start_server(&self) {
        println!("Starting server at: {} !", self.address);
        let listener = TcpListener::bind(self.address).unwrap();

        for stream in listener.incoming() {
            let server_clone = Arc::clone(&self.server);

            match stream {
                Ok(stream) => {
                    thread::spawn(move || handle_connection(Arc::clone(&server_clone), stream));
                }
                Err(e) => {
                    println!("Error while listening to client: {}", e);
                }
            }
        }
    }
}

fn handle_connection(server: Arc<Mutex<Server>>, mut stream: TcpStream) {
    loop {
        let mut buffer = [0; 256];
        stream.read(&mut buffer).unwrap();

        let deserialized: RpcMessage = bincode::deserialize(&buffer).unwrap();
        let server_clone = Arc::clone(&server);

        let response = match deserialized {
            RpcMessage::Heartbeat { term, peer_id } => {
                handle_log_entry(server_clone, term, peer_id)
            }
            RpcMessage::VoteRequest { term, candidate_id } => {
                handle_vote_request(server_clone, term, candidate_id)
            }
            _ => Vec::new(), // Response messages;
        };

        stream.write(&response).unwrap();
        stream.flush().unwrap();
    }
}

fn handle_log_entry(server: Arc<Mutex<Server>>, term: u64, peer_id: String) -> Vec<u8> {
    let server_clone = Arc::clone(&server);
    let term = crate::raft::core::handle_log_entry(
        server_clone,
        LogEntry::Heartbeat {
            term: term,
            peer_id: peer_id.to_string(),
        },
    );

    let response = RpcMessage::HeartbeatResponse {
        term: term,
        peer_id: peer_id,
    };

    bincode::serialize(&response).unwrap()
}

fn handle_vote_request(server: Arc<Mutex<Server>>, term: u64, candidate_id: String) -> Vec<u8> {
    let server_clone = Arc::clone(&server);
    let response = crate::raft::core::handle_vote_request(
        server_clone,
        VoteRequest {
            term: term,
            candidate_id: candidate_id,
        },
    );

    let response = RpcMessage::VoteResponse {
        term: response.term,
        vote_granted: response.vote_granted,
    };

    bincode::serialize(&response).unwrap()
}
