use std::collections::HashMap;
use std::marker::PhantomData;

use failure;

use log::LogEntry;
use rpc;
use {MilliSec, NodeId, Term};

pub struct Node<T, R, S>
where
    R: rpc::Recv<T>,
    S: rpc::Send<T>,
{
    // Persistent state on all nodes
    // Unique ID of each node
    id: NodeId,
    /// Latest term server has seen
    term: Term,

    /// Candidate ID that received vote in current term
    voted_for: Option<NodeId>,
    /// Log entries
    log: Vec<LogEntry<T>>,

    // Volatile state on all nodes
    /// State of each node
    state: NodeState,

    /// INdex of highest log entry known to be committed
    commit_idx: usize,
    /// Index of highest log entry applied to state machine
    last_applied: usize,

    // Volatile state on leaders
    /// For each server, index of the next log entry to send to that server
    next_idx: Vec<usize>,
    /// For each server, index of highest log entry known to be replicated on server
    match_idx: Vec<usize>,

    election_timeout_range: (MilliSec, MilliSec),

    // Message
    recv: R,
    peers: HashMap<NodeId, S>,
}

impl<T, R, S> Node<T, R, S>
where
    R: rpc::Recv<T>,
    S: rpc::Send<T>,
{
    pub fn new(cfg: NodeConfig<T, R, S>) -> Result<Self, failure::Error> {
        Ok(Node {
            id: cfg.id,
            term: 0,

            voted_for: None,
            log: vec![],

            state: NodeState::Follower,

            commit_idx: 0,
            last_applied: 0,

            next_idx: vec![],
            match_idx: vec![],

            election_timeout_range: cfg.election_timeout_range,

            recv: cfg.recv,
            peers: cfg.peers,
        })
    }
}

pub enum NodeState {
    Leader,
    Candidate,
    Follower,
}

#[derive(Builder, Debug)]
pub struct NodeConfig<T, R, S>
where
    R: rpc::Recv<T>,
    S: rpc::Send<T>,
{
    id: NodeId,
    recv: R,
    peers: HashMap<NodeId, S>,
    election_timeout_range: (MilliSec, MilliSec),
    _marker: PhantomData<T>,
}
