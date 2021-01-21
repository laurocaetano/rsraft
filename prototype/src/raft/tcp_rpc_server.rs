use crate::core::{RpcMessage, Server};
use crate::RpcServer;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub struct TcpRpcServer {
    server: Arc<Mutex<Server>>,
    address: SocketAddrV4,
}

impl TcpRpcServer {
    pub fn new(server: Arc<Mutex<Server>>, address: SocketAddrV4) -> Self {
        TcpRpcServer {
            server: Arc::clone(&server),
            address: address,
        }
    }
}

impl RpcServer for TcpRpcServer {
    fn start_server(&self) {
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
    println!("Received log entry from {}, with term {}", peer_id, term);
    let server_clone = Arc::clone(&server);
    let term = crate::core::handle_heartbeat(
        server_clone,
        crate::core::LogEntry::Heartbeat {
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
    let response = crate::core::handle_vote_request(
        server_clone,
        crate::core::VoteRequest {
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
