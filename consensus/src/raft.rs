use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rand::Rng;

use crate::{Result, ConsensusError, NodeId, NodeInfo, NodeStatus, ClusterView};
use crate::messages::*;

#[derive(Debug, Clone)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

pub struct RaftNode {
    node_id: NodeId,
    state: Arc<RwLock<RaftState>>,
    current_term: Arc<RwLock<u64>>,
    voted_for: Arc<RwLock<Option<NodeId>>>,
    log: Arc<RwLock<Vec<LogEntry>>>,
    commit_index: Arc<RwLock<u64>>,
    last_applied: Arc<RwLock<u64>>,
    
    // Leader state
    next_index: Arc<RwLock<HashMap<NodeId, u64>>>,
    match_index: Arc<RwLock<HashMap<NodeId, u64>>>,
    
    // Cluster management
    cluster_nodes: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
    leader: Arc<RwLock<Option<NodeId>>>,
    
    // Timers and configuration
    election_timeout: Arc<Mutex<Option<tokio::time::Instant>>>,
    heartbeat_interval: tokio::time::Duration,
    config: crate::Config,
}

impl RaftNode {
    pub async fn new(node_id: NodeId, config: crate::Config) -> Result<Self> {
        let heartbeat_interval = tokio::time::Duration::from_millis(config.heartbeat_interval_ms);
        
        Ok(Self {
            node_id,
            state: Arc::new(RwLock::new(RaftState::Follower)),
            current_term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(Vec::new())),
            commit_index: Arc::new(RwLock::new(0)),
            last_applied: Arc::new(RwLock::new(0)),
            next_index: Arc::new(RwLock::new(HashMap::new())),
            match_index: Arc::new(RwLock::new(HashMap::new())),
            cluster_nodes: Arc::new(RwLock::new(HashMap::new())),
            leader: Arc::new(RwLock::new(None)),
            election_timeout: Arc::new(Mutex::new(None)),
            heartbeat_interval,
            config,
        })
    }

    pub async fn run(&self) -> Result<()> {
        self.reset_election_timeout().await;
        
        let election_task = {
            let node = self.clone_for_task();
            tokio::spawn(async move {
                node.election_timeout_loop().await
            })
        };

        let heartbeat_task = {
            let node = self.clone_for_task();
            tokio::spawn(async move {
                node.leader_heartbeat_loop().await
            })
        };

        tokio::select! {
            result = election_task => {
                tracing::error!("Election task failed: {:?}", result);
                Err(ConsensusError::ElectionTimeout)
            }
            result = heartbeat_task => {
                tracing::error!("Heartbeat task failed: {:?}", result);
                Err(ConsensusError::Network("Heartbeat task failed".to_string()))
            }
        }
    }

    pub async fn handle_heartbeat(&self, heartbeat: HeartbeatMessage) -> Result<()> {
        let current_term = *self.current_term.read().await;
        
        if heartbeat.term >= current_term {
            *self.current_term.write().await = heartbeat.term;
            *self.state.write().await = RaftState::Follower;
            *self.leader.write().await = Some(heartbeat.from.clone());
            self.reset_election_timeout().await;
        }
        
        Ok(())
    }

    pub async fn handle_vote_request(&self, vote_request: VoteRequest) -> Result<()> {
        let mut current_term = self.current_term.write().await;
        let mut voted_for = self.voted_for.write().await;
        
        let grant_vote = if vote_request.term > *current_term {
            *current_term = vote_request.term;
            *voted_for = None;
            true
        } else if vote_request.term == *current_term {
            voted_for.is_none() || *voted_for == Some(vote_request.candidate_id.clone())
        } else {
            false
        };

        if grant_vote {
            *voted_for = Some(vote_request.candidate_id.clone());
            self.reset_election_timeout().await;
        }

        // TODO: Send vote response via network layer
        tracing::debug!("Vote request from {:?}: granted={}", vote_request.candidate_id, grant_vote);
        
        Ok(())
    }

    pub async fn handle_vote_response(&self, vote_response: VoteResponse) -> Result<()> {
        let state = self.state.read().await.clone();
        if !matches!(state, RaftState::Candidate) {
            return Ok(());
        }

        if vote_response.vote_granted {
            // TODO: Count votes and become leader if majority
            tracing::debug!("Received vote from {:?}", vote_response.from);
        }

        Ok(())
    }

    pub async fn handle_append_entries(&self, append_request: AppendEntriesRequest) -> Result<()> {
        let current_term = *self.current_term.read().await;
        
        if append_request.term >= current_term {
            *self.current_term.write().await = append_request.term;
            *self.state.write().await = RaftState::Follower;
            *self.leader.write().await = Some(append_request.leader_id);
            self.reset_election_timeout().await;
            
            // TODO: Implement log consistency checks and append entries
            tracing::debug!("Append entries from leader {:?}", append_request.leader_id);
        }

        Ok(())
    }

    pub async fn handle_append_entries_response(&self, _response: AppendEntriesResponse) -> Result<()> {
        // TODO: Update next_index and match_index for follower
        Ok(())
    }

    pub async fn handle_replication_request(&self, request: ReplicationRequest) -> Result<()> {
        // TODO: Handle object replication request
        tracing::debug!("Replication request for object {}", request.object_id);
        Ok(())
    }

    pub async fn handle_replication_response(&self, _response: ReplicationResponse) -> Result<()> {
        // TODO: Handle replication response
        Ok(())
    }

    pub async fn handle_join_request(&self, _request: JoinRequest) -> Result<()> {
        // TODO: Handle node join request
        Ok(())
    }

    pub async fn handle_join_response(&self, _response: JoinResponse) -> Result<()> {
        // TODO: Handle join response
        Ok(())
    }

    pub async fn submit_replication_request(&self, request: ReplicationRequest) -> Result<()> {
        let state = self.state.read().await.clone();
        if !matches!(state, RaftState::Leader) {
            let leader = self.leader.read().await.clone();
            return Err(ConsensusError::NotLeader(leader));
        }

        // TODO: Create log entry and replicate to followers
        tracing::debug!("Submitting replication request for object {}", request.object_id);
        
        Ok(())
    }

    pub async fn get_cluster_view(&self) -> ClusterView {
        let nodes = self.cluster_nodes.read().await;
        let leader = self.leader.read().await.clone();
        let term = *self.current_term.read().await;
        
        ClusterView {
            nodes: nodes.values().cloned().collect(),
            leader,
            term,
        }
    }

    pub async fn is_leader(&self) -> bool {
        matches!(*self.state.read().await, RaftState::Leader)
    }

    pub async fn get_current_leader(&self) -> Option<NodeId> {
        self.leader.read().await.clone()
    }

    async fn start_election(&self) -> Result<()> {
        tracing::info!("Starting election for term {}", *self.current_term.read().await + 1);
        
        *self.state.write().await = RaftState::Candidate;
        *self.current_term.write().await += 1;
        *self.voted_for.write().await = Some(self.node_id.clone());
        
        self.reset_election_timeout().await;
        
        // TODO: Send vote requests to all known nodes
        // TODO: Count votes and become leader if majority
        
        Ok(())
    }

    async fn become_leader(&self) -> Result<()> {
        tracing::info!("Becoming leader for term {}", *self.current_term.read().await);
        
        *self.state.write().await = RaftState::Leader;
        *self.leader.write().await = Some(self.node_id.clone());
        
        // Initialize leader state
        let mut next_index = self.next_index.write().await;
        let mut match_index = self.match_index.write().await;
        let last_log_index = self.log.read().await.len() as u64;
        
        let cluster_nodes = self.cluster_nodes.read().await;
        for node_id in cluster_nodes.keys() {
            if *node_id != self.node_id {
                next_index.insert(node_id.clone(), last_log_index + 1);
                match_index.insert(node_id.clone(), 0);
            }
        }
        
        Ok(())
    }

    async fn reset_election_timeout(&self) {
        let timeout_ms = rand::thread_rng().gen_range(
            self.config.consensus_timeout_ms..self.config.consensus_timeout_ms * 2
        );
        let timeout = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        
        *self.election_timeout.lock().await = Some(timeout);
    }

    async fn election_timeout_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            let timeout_guard = self.election_timeout.lock().await;
            if let Some(timeout) = *timeout_guard {
                if tokio::time::Instant::now() >= timeout {
                    drop(timeout_guard);
                    
                    let state = self.state.read().await.clone();
                    if !matches!(state, RaftState::Leader) {
                        self.start_election().await?;
                    }
                }
            }
        }
    }

    async fn leader_heartbeat_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(self.heartbeat_interval);
        
        loop {
            interval.tick().await;
            
            let state = self.state.read().await.clone();
            if matches!(state, RaftState::Leader) {
                // TODO: Send heartbeats to all followers
                tracing::trace!("Sending leader heartbeat");
            }
        }
    }

    fn clone_for_task(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            state: self.state.clone(),
            current_term: self.current_term.clone(),
            voted_for: self.voted_for.clone(),
            log: self.log.clone(),
            commit_index: self.commit_index.clone(),
            last_applied: self.last_applied.clone(),
            next_index: self.next_index.clone(),
            match_index: self.match_index.clone(),
            cluster_nodes: self.cluster_nodes.clone(),
            leader: self.leader.clone(),
            election_timeout: self.election_timeout.clone(),
            heartbeat_interval: self.heartbeat_interval,
            config: self.config.clone(),
        }
    }
}