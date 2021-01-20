use crate::core::RpcMessage;
use crate::RpcServer;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::thread;
use uuid::Uuid;

pub struct TcpRpcServer;

impl RpcServer for TcpRpcServer {
    fn start_server(&self, address: SocketAddrV4) {
        println!("Starting server at: {} !", address);
        let listener = TcpListener::bind(address).unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    thread::spawn(move || handle_connection(stream));
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    loop {
        let mut buffer = [0; 256];
        stream.read(&mut buffer).unwrap();
        println!("reading...");

        let deserialized: RpcMessage = bincode::deserialize(&buffer).unwrap();

        let response = match deserialized {
            RpcMessage::Heartbeat { term, peer_id } => handle_log_entry(term, peer_id),
            RpcMessage::VoteRequest { term, candidate_id } => {
                handle_vote_request(term, candidate_id)
            }
            _ => Vec::new(), // Response messages;
        };

        stream.write(&response).unwrap();
        stream.flush().unwrap();
        println!("Responded");
    }
}

fn handle_log_entry(term: u64, candidate_id: Uuid) -> Vec<u8> {
    println!("LOG");
    let response = RpcMessage::HeartbeatResponse {
        term: term,
        peer_id: candidate_id,
    };

    bincode::serialize(&response).unwrap()
}

fn handle_vote_request(term: u64, candidate_id: Uuid) -> Vec<u8> {
    println!("VoteRequest");
    let response = RpcMessage::VoteResponse {
        term: term,
        vote_granted: true,
    };
    bincode::serialize(&response).unwrap()
}
