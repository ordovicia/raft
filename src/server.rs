use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

// use failure;
use rand::{self, Rng};

use log::LogEntry;
use message::Message;
use {Millisec, ServerId, Term};

pub trait ServerSend<T> {
    /// Send a message to this server asynchronously
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>>;
}

#[derive(Clone, Debug)]
pub struct RemoteServer<T> {
    /// Unique ID of each server
    id: ServerId,

    /// Sending-half of channel
    tx: mpsc::Sender<Message<T>>,
}

impl<T> RemoteServer<T> {
    pub fn new(id: ServerId, tx: mpsc::Sender<Message<T>>) -> Self {
        RemoteServer { id, tx }
    }
}

impl<T> ServerSend<T> for RemoteServer<T> {
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>> {
        self.tx.send(msg)
    }
}

pub trait ServerRecv<T> {
    /// Attempts to receive a message.
    /// Returns `Err(mpsc::TryRecvError::Empty)` if not exists.
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError>;

    /// Attempts to wait for a message.
    /// Returns `Err(mpsc::RecvTimeoutError::Timeout)` if it waits more than `timeout`.
    fn recv_timeout(&self, timeout: Duration) -> Result<Message<T>, mpsc::RecvTimeoutError>;
}

#[derive(Debug)]
pub struct LocalServer<T> {
    // Persistent state on all servers
    /// Unique ID of each server
    id: ServerId,
    /// Latest term server has seen
    term: Term,

    /// Candidate ID that received vote in current term
    voted_for: Option<ServerId>,
    /// Log entries
    log: Vec<LogEntry<T>>,

    // Volatile state on all servers
    /// State of each server
    state: ServerState,

    /// Index of highest log entry known to be committed
    commit_idx: usize,
    /// Index of highest log entry applied to state machine
    last_applied: usize,

    // Volatile state on leaders
    /// For each server, index of the next log entry to send to that server
    next_idx: Vec<usize>,
    /// For each server, index of highest log entry known to be replicated on server
    match_idx: Vec<usize>,

    election_timeout_range: (Millisec, Millisec),

    // Message
    rx: mpsc::Receiver<Message<T>>,
    peers: HashMap<ServerId, RemoteServer<T>>,
}

impl<T> LocalServer<T> {
    pub fn new(
        id: ServerId,
        election_timeout_range: (Millisec, Millisec),
    ) -> (Self, mpsc::Sender<Message<T>>) {
        let (tx, rx) = mpsc::channel();

        let server = LocalServer {
            id,
            term: 0,

            voted_for: None,
            log: vec![],

            state: ServerState::Follower,

            commit_idx: 0,
            last_applied: 0,

            next_idx: vec![],
            match_idx: vec![],

            election_timeout_range,

            rx,
            peers: HashMap::new(),
        };

        (server, tx)
    }

    pub fn set_peers(&mut self, peers: HashMap<ServerId, RemoteServer<T>>) {
        self.peers = peers;
    }

    fn election_time(&self) -> Millisec {
        rand::thread_rng().gen_range(self.election_timeout_range.0, self.election_timeout_range.1)
    }
}

impl<T> ServerRecv<T> for LocalServer<T> {
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<Message<T>, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

/// State of each server
#[derive(Clone, Debug)]
pub enum ServerState {
    Leader,
    Candidate,
    Follower,
}
