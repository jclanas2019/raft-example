use async_raft::storage::{HardState, InitialState, RaftStorage, CurrentSnapshotData};
use async_raft::raft::{Entry, MembershipConfig};
use async_raft::{AppData, AppDataResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyAppData(pub String);

impl AppData for MyAppData {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyAppResponse {
    pub success: bool,
    pub data: Option<String>,
}

impl AppDataResponse for MyAppResponse {}

#[derive(Debug, Clone)]
pub struct MemoryStore {
    pub logs: Arc<RwLock<HashMap<u64, Entry<MyAppData>>>>,
    pub state: Arc<RwLock<Option<HardState>>>,
    pub membership: Arc<RwLock<MembershipConfig>>,
    pub data_store: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(None)),
            membership: Arc::new(RwLock::new(MembershipConfig::new_initial(1))),
            data_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn read_data(&self, key: &str) -> Option<String> {
        let store = self.data_store.read().await;
        store.get(key).cloned()
    }

    pub async fn write_data(&self, key: String, value: String) -> bool {
        let mut store = self.data_store.write().await;
        store.insert(key, value);
        true
    }
}

#[async_trait]
impl RaftStorage<MyAppData, MyAppResponse> for MemoryStore {
    type Snapshot = Cursor<Vec<u8>>;
    type ShutdownError = std::io::Error;

    async fn get_initial_state(&self) -> Result<InitialState, anyhow::Error> {
        let hard_state = self.state.read().await.clone().unwrap_or_else(|| HardState {
            current_term: 0,
            voted_for: None,
        });
        let membership = self.membership.read().await.clone();
        Ok(InitialState {
            hard_state,
            membership,
            last_applied_log: 0,
            last_log_index: 0,
            last_log_term: 0,
        })
    }

    async fn save_hard_state(&self, hs: &HardState) -> Result<(), anyhow::Error> {
        let mut state = self.state.write().await;
        *state = Some(hs.clone());
        Ok(())
    }

    async fn get_log_entries(&self, start: u64, stop: u64) -> Result<Vec<Entry<MyAppData>>, anyhow::Error> {
        let logs = self.logs.read().await;
        Ok((start..stop).filter_map(|i| logs.get(&i).cloned()).collect())
    }

    async fn append_entry_to_log(&self, entry: &Entry<MyAppData>) -> Result<(), anyhow::Error> {
        let mut logs = self.logs.write().await;
        logs.insert(entry.index, entry.clone());
        Ok(())
    }

    async fn replicate_to_log(&self, entries: &[Entry<MyAppData>]) -> Result<(), anyhow::Error> {
        let mut logs = self.logs.write().await;
        for entry in entries {
            logs.insert(entry.index, entry.clone());
        }
        Ok(())
    }

    async fn apply_entry_to_state_machine(
        &self,
        _index: &u64,
        data: &MyAppData,
    ) -> Result<MyAppResponse, anyhow::Error> {
        let command_parts: Vec<&str> = data.0.splitn(3, ' ').collect();
        
        match command_parts.get(0).map(|s| *s) {
            Some("SET") => {
                if command_parts.len() >= 3 {
                    let key = command_parts[1].to_string();
                    let value = command_parts[2].to_string();
                    let success = self.write_data(key, value).await;
                    Ok(MyAppResponse { success, data: None })
                } else {
                    Ok(MyAppResponse { 
                        success: false, 
                        data: Some("SET command requires key and value".to_string()) 
                    })
                }
            }
            Some("GET") => {
                if command_parts.len() >= 2 {
                    let key = command_parts[1];
                    let data = self.read_data(key).await;
                    Ok(MyAppResponse { 
                        success: true, 
                        data 
                    })
                } else {
                    Ok(MyAppResponse { 
                        success: false, 
                        data: Some("GET command requires a key".to_string()) 
                    })
                }
            }
            _ => Ok(MyAppResponse { 
                success: false, 
                data: Some("Unknown command".to_string()) 
            })
        }
    }

    async fn replicate_to_state_machine(&self, _entries: &[(&u64, &MyAppData)]) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn get_current_snapshot(&self) -> Result<Option<CurrentSnapshotData<Self::Snapshot>>, anyhow::Error> {
        Ok(None)
    }

    async fn create_snapshot(&self) -> Result<(String, Box<Self::Snapshot>), anyhow::Error> {
        Ok(("snapshot".to_string(), Box::new(Cursor::new(Vec::new()))))
    }

    async fn finalize_snapshot_installation(
        &self,
        _index: u64,
        _term: u64,
        _delete_through: Option<u64>,
        _id: String,
        _snapshot: Box<Self::Snapshot>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn get_membership_config(&self) -> Result<MembershipConfig, anyhow::Error> {
        Ok(self.membership.read().await.clone())
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> Result<(), anyhow::Error> {
        let mut logs = self.logs.write().await;
        if let Some(stop) = stop {
            logs.retain(|&index, _| index < start || index >= stop);
        } else {
            logs.retain(|&index, _| index < start);
        }
        Ok(())
    }

    async fn do_log_compaction(&self) -> Result<CurrentSnapshotData<Self::Snapshot>, anyhow::Error> {
        Ok(CurrentSnapshotData {
            term: 0,
            index: 0,
            membership: self.membership.read().await.clone(),
            snapshot: Box::new(Cursor::new(Vec::new())),
        })
    }
}
