use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq)]
pub enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LogEntry {
    Heartbeat { term: u64, peer_id: String },
}

#[derive(Debug)]
pub struct Peer {
    pub id: String,
    pub address: SocketAddrV4,
}

#[derive(Debug)]
pub struct Leader {
    pub id: String,
    pub term: u64,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub timeout: Duration,
}

#[derive(Debug)]
pub struct Server {
    id: String,
    address: SocketAddrV4,
    state: State,
    term: u64,
    log_entries: Vec<LogEntry>,
    voted_for: Option<Peer>,
    next_timeout: Option<Instant>,
    config: ServerConfig,
    current_leader: Option<Leader>,
    number_of_peers: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: String,
}

pub struct VoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

pub trait RpcClient {
    fn request_vote(&self, request: VoteRequest) -> Vec<VoteResponse>;

    fn broadcast_log_entry(&self, log_entry: LogEntry);
}
