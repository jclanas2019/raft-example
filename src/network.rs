use async_raft::raft::{
    AppendEntriesRequest, 
    AppendEntriesResponse, 
    InstallSnapshotRequest,
    InstallSnapshotResponse,
    VoteRequest, 
    VoteResponse
};
use async_raft::RaftNetwork;
use async_trait::async_trait;
use crate::store::MyAppData;

#[derive(Clone)]
pub struct Network {}

#[async_trait]
impl RaftNetwork<MyAppData> for Network {
    async fn append_entries(
        &self,
        _target: u64,
        _rpc: AppendEntriesRequest<MyAppData>,
    ) -> Result<AppendEntriesResponse, anyhow::Error> {
        Ok(AppendEntriesResponse {
            term: 0,
            success: true,
            conflict_opt: None,
        })
    }

    async fn install_snapshot(
        &self,
        _target: u64,
        _rpc: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse, anyhow::Error> {
        Ok(InstallSnapshotResponse { term: 0 })
    }

    async fn vote(
        &self,
        _target: u64,
        _rpc: VoteRequest,
    ) -> Result<VoteResponse, anyhow::Error> {
        Ok(VoteResponse {
            term: 0,
            vote_granted: true,
        })
    }
}