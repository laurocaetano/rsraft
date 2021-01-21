use crate::core::RpcMessage;
use crate::core::{LogEntry, VoteRequest, VoteRequestResponse};
use crate::core::{Peer, RpcClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::net::TcpStream;

pub struct TcpRpcClient {
    servers: HashMap<String, TcpStream>,
}

impl RpcClient for TcpRpcClient {
    fn request_vote(&self, request: VoteRequest) -> Vec<VoteRequestResponse> {
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
                response.push(VoteRequestResponse {
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
            let mut rpc_responses: Vec<RpcMessage> = Vec::new();

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
