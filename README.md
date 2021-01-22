# rsraft

![Rust](https://github.com/laurocaetano/rsraft/workflows/Rust/badge.svg)

Raft implementation in Rust.

The aim of this project is implementing the Raft Consensus Algorithm as described in the [paper](https://raft.github.io/raft.pdf), with the goal of fully understanding the nuances of the algorithm, while at the same time, learning the Rust language in practice.

## Development

`$ cargo build`
`$ cargo test`

:warning: This project is in no way intended for production usage! :warning:

## Current status

The project is now implementing Raft's leader election and the setup is demonstrated in the `demo` file under `src/raft` folder.

The goal was never to fully implement Raft, but rather have a feeling of how it would be to implement it, and also learn Rust.

When I have more free time to dedicate to this project, I will fully implement the rest of the algorithm. 

## Running the demo app

In order to see the algorithm in action, one can run the demo app, by simply:

`$ cargo run`

The output would look like this:

```
22:46:10 [INFO] Starting server at: 127.0.0.1:3300...
22:46:10 [INFO] Starting server at: 127.0.0.1:3301...
22:46:10 [INFO] Starting server at: 127.0.0.1:3302...
22:46:11 [INFO] The server server_1, has a timeout of 3 seconds.
22:46:11 [INFO] The server server_3, has a timeout of 6 seconds.
22:46:11 [INFO] The server server_2, has a timeout of 5 seconds.
22:46:14 [INFO] Server server_1 has timed out.
22:46:14 [INFO] Server server_1, with term 1, started the election process.
22:46:14 [INFO] Server server_1 has won the election! The new term is: 1
22:46:14 [INFO] Server server_3 with term 0, received heartbeat from server_1 with term 1
22:46:14 [INFO] Server server_3 becoming follower. The new leader is: server_1
22:46:14 [INFO] Server server_2 with term 0, received heartbeat from server_1 with term 1
22:46:14 [INFO] Server server_2 becoming follower. The new leader is: server_1
22:46:14 [INFO] Server server_3 with term 1, received heartbeat from server_1 with term 1
22:46:14 [INFO] Server server_2 with term 1, received heartbeat from server_1 with term 1
22:46:16 [INFO] Server server_3 with term 1, received heartbeat from server_1 with term 1
22:46:16 [INFO] Server server_2 with term 1, received heartbeat from server_1 with term 1
```
