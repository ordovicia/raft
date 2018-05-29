use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

// use failure;
use rand::{self, Rng};

use log::LogEntry;
use message::Message;
use {Millisec, NodeId, Term};

pub trait NodeSend<T> {
    /// Send a message to this node asynchronously
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>>;
}

#[derive(Clone, Debug)]
pub struct RemoteNode<T> {
    /// Unique ID of each node
    id: NodeId,

    /// Sending-half of channel
    tx: mpsc::Sender<Message<T>>,
}

impl<T> RemoteNode<T> {
    pub fn new(id: NodeId, tx: mpsc::Sender<Message<T>>) -> Self {
        RemoteNode { id, tx }
    }
}

impl<T> NodeSend<T> for RemoteNode<T> {
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>> {
        self.tx.send(msg)
    }
}

pub trait NodeRecv<T> {
    /// Attempts to receive a message.
    /// Returns `Err(mpsc::TryRecvError::Empty)` if not exists.
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError>;

    /// Attempts to wait for a message.
    /// Returns `Err(mpsc::RecvTimeoutError::Timeout)` if it waits more than `timeout`.
    fn recv_timeout(&self, timeout: Duration) -> Result<Message<T>, mpsc::RecvTimeoutError>;
}

#[derive(Debug)]
pub struct LocalNode<T> {
    // Persistent state on all nodes
    /// Unique ID of each node
    id: NodeId,
    /// Latest term node has seen
    term: Term,

    /// Candidate ID that received vote in current term
    voted_for: Option<NodeId>,
    /// Log entries
    log: Vec<LogEntry<T>>,

    // Volatile state on all nodes
    /// State of each node
    state: NodeState,

    /// Index of highest log entry known to be committed
    commit_idx: usize,
    /// Index of highest log entry applied to state machine
    last_applied: usize,

    // Volatile state on leaders
    /// For each node, index of the next log entry to send to that node
    next_idx: Vec<usize>,
    /// For each node, index of highest log entry known to be replicated on node
    match_idx: Vec<usize>,

    election_timeout_range: (Millisec, Millisec),

    // Message
    rx: mpsc::Receiver<Message<T>>,
    peers: HashMap<NodeId, RemoteNode<T>>,
}

impl<T> LocalNode<T> {
    pub fn new(
        id: NodeId,
        election_timeout_range: (Millisec, Millisec),
    ) -> (Self, mpsc::Sender<Message<T>>) {
        let (tx, rx) = mpsc::channel();

        let node = LocalNode {
            id,
            term: 0,

            voted_for: None,
            log: vec![],

            state: NodeState::Follower,

            commit_idx: 0,
            last_applied: 0,

            next_idx: vec![],
            match_idx: vec![],

            election_timeout_range,

            rx,
            peers: HashMap::new(),
        };

        (node, tx)
    }

    pub fn set_peers(&mut self, peers: HashMap<NodeId, RemoteNode<T>>) {
        self.peers = peers;
    }

    fn election_time(&self) -> Millisec {
        rand::thread_rng().gen_range(self.election_timeout_range.0, self.election_timeout_range.1)
    }
}

impl<T> NodeRecv<T> for LocalNode<T> {
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<Message<T>, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

/// State of each node
#[derive(Clone, Debug)]
pub enum NodeState {
    Leader,
    Candidate,
    Follower,
}
