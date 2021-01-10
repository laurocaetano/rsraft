use uuid::Uuid;

#[derive(Debug, PartialEq)]
enum State {
    FOLLOWER,
    LEADER,
    CANDIDATE,
}

pub struct Server {
    id: Uuid,
    state: State,
    term: u64,
}

impl Server {
    fn new() -> Self {
        Server {
            id: Uuid::new_v4(),
            state: State::FOLLOWER,
            term: 0,
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
    }
}
