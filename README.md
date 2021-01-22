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
