mod store;
mod network;

use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::io::AsyncWriteExt;
use async_raft::Config;
use async_raft::Raft;
use async_raft::raft::ClientWriteRequest;

use store::{MemoryStore, MyAppData};
use network::Network;

#[tokio::main]
async fn main() {
    let id = 1;

    let store = Arc::new(MemoryStore::new());
    let network = Arc::new(Network {});
    let config = Arc::new(
        Config::build("test-cluster".to_string())
            .validate()
            .expect("Invalid config"),
    );

    let raft = Arc::new(Raft::new(id, config, network, store));

    // Forzar el nodo a actuar como líder
    tokio::spawn({
        let raft_clone = raft.clone();
        async move {
            let members: HashSet<u64> = vec![id].into_iter().collect();
            raft_clone.initialize(members).await.expect("Failed to initialize cluster");
        }
    });

    println!("Node {} started. Available commands:", id);
    println!("  SET <key> <value>");
    println!("  GET <key>");
    println!("Type 'exit' to quit.");

    let mut stdout = io::stdout();
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    loop {
        stdout.write_all(b"> ").await.unwrap();
        stdout.flush().await.unwrap();

        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                println!("Error reading input: {:?}", e);
                break;
            }
        };

        let trimmed_line = line.trim();
        if trimmed_line.eq_ignore_ascii_case("exit") {
            break;
        }

        let command = trimmed_line.to_string();
        let payload = MyAppData(command);
        let request = ClientWriteRequest::new(payload);

        match raft.client_write(request).await {
            Ok(response) => {
                if response.data.success {
                    match response.data.data {
                        Some(data) => println!("{}", data),
                        None => println!("Operation completed successfully"),
                    }
                } else {
                    println!("Operation failed: {}", 
                        response.data.data.unwrap_or_else(|| "Unknown error".to_string()));
                }
            },
            Err(error) => println!("Operation failed: {:?}", error),
        }
    }

    println!("Shutting down node {}.", id);
}
