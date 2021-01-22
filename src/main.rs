mod raft;
use log::LevelFilter;
use simplelog::{Config, TermLogger, TerminalMode};

fn main() {
    TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Stdout).unwrap();
    crate::raft::demo::start_demo();
}
