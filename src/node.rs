use std::cmp;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// use failure;
use rand::{self, Rng};

use error::Error;
use log::{LogEntry, LogEntryId};
use message::{AppendEntries, Message, RequestVote};
use {Millisec, NodeId, Term};

pub trait Remote<T: Clone> {
    /// Send a message to this node asynchronously
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>>;
}

#[derive(Clone, Debug)]
pub struct RemoteNode<T: Clone> {
    /// Unique ID of each node
    id: NodeId,

    /// Sending-half of channel
    tx: mpsc::Sender<Message<T>>,
}

impl<T: Clone> RemoteNode<T> {
    pub fn new(id: NodeId, tx: mpsc::Sender<Message<T>>) -> Self {
        RemoteNode { id, tx }
    }
}

impl<T: Clone> Remote<T> for RemoteNode<T> {
    fn send(&self, msg: Message<T>) -> Result<(), mpsc::SendError<Message<T>>> {
        self.tx.send(msg)
    }
}

pub trait Local<T: Clone> {
    /// Attempts to receive a message.
    /// Returns `Err(mpsc::TryRecvError::Empty)` if not exists.
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError>;

    /// Attempts to wait for a message.
    /// Returns `Err(mpsc::RecvTimeoutError::Timeout)` if `deadline` is reached.
    fn recv_deadline(&self, deadline: Instant) -> Result<Message<T>, mpsc::RecvTimeoutError>;
}

#[derive(Debug)]
pub struct LocalNode<T: Clone, R: Remote<T>> {
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

    heartbeat_tick: Millisec,
    election_timeout_range: (Millisec, Millisec),

    // Channels
    rx: mpsc::Receiver<Message<T>>,
    peers: HashMap<NodeId, R>,
}

impl<T: Clone, R: Remote<T>> LocalNode<T, R> {
    pub fn new(
        id: NodeId,
        election_timeout_range: (Millisec, Millisec),
    ) -> (Self, mpsc::Sender<Message<T>>) {
        let (tx, rx) = mpsc::channel();

        assert!(election_timeout_range.0 <= election_timeout_range.1);

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

            heartbeat_tick: election_timeout_range.0 / 2,
            election_timeout_range,

            rx,
            peers: HashMap::new(),
        };

        (node, tx)
    }

    pub fn set_peers(&mut self, peers: HashMap<NodeId, R>) {
        self.peers = peers;
    }

    pub fn tick_one(&mut self) -> Result<(), Error> {
        self.voted_for = None;

        match self.state {
            NodeState::Follower => self.tick_follower(),
            NodeState::Candidate => self.tick_candidate(),
            NodeState::Leader => self.tick_leader(),
        }
    }

    fn tick_follower(&mut self) -> Result<(), Error> {
        loop {
            let deadline = Instant::now() + Duration::from_millis(self.election_timeout());

            match self.recv_deadline(deadline) {
                Ok(msg) => {
                    match msg {
                        Message::RequestVote(msg) => {
                            self.term = cmp::max(msg.term, self.term);
                            self.res_request_vote(&msg)?;
                        }
                        Message::RequestVoteRes { term, .. } => {
                            self.term = cmp::max(term, self.term);
                            // ignore
                        }
                        Message::AppendEntries(msg) => {
                            self.term = cmp::max(msg.term, self.term);
                            self.res_append_entries(&msg)?;

                            if self.commit_idx > self.last_applied {
                                self.last_applied = self.commit_idx;
                                // TODO: commit
                            }
                        }
                        Message::AppendEntriesRes { term, .. } => {
                            self.term = cmp::max(term, self.term);
                            // ignore
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Election timeout
                    self.state = NodeState::Candidate;
                    break Ok(());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(Error::RxDisconnected);
                }
            }
        }
    }

    fn tick_candidate(&mut self) -> Result<(), Error> {
        self.term += 1;

        // Sends RequestVote to all nodes
        for peer in self.peers.values() {
            if let Err(mpsc::SendError(_)) = peer.send(Message::RequestVote(RequestVote {
                term: self.term,
                candidate_id: self.id,
                latest_log_id: self.log.last().map(|l| l.id),
            })) {
                return Err(Error::TxDisconnected);
            }
        }

        let deadline = Instant::now() + Duration::from_millis(self.election_timeout());
        let mut voted_cnt = 1;

        // Receives RequestVoteRes until election_timeout
        loop {
            match self.recv_deadline(deadline) {
                Ok(msg) => {
                    match msg {
                        Message::RequestVote(msg) => {
                            if msg.term > self.term {
                                // In old term
                                // Converts to follower and responds to the msg
                                self.term = msg.term;
                                self.state = NodeState::Follower;
                                self.res_request_vote(&msg)?;

                                break Ok(());
                            }
                        }

                        Message::RequestVoteRes { term, vote_granted } => {
                            if term > self.term {
                                // In old term
                                // Converts to follower
                                self.term = term;
                                self.state = NodeState::Follower;

                                break Ok(());
                            }

                            if vote_granted {
                                voted_cnt += 1;
                            }

                            if voted_cnt >= self.majority() {
                                // Wins majority
                                // Converts to leader
                                self.state = NodeState::Leader;
                                break Ok(());
                            }
                        }

                        Message::AppendEntries(msg) => {
                            if msg.term > self.term {
                                self.term = msg.term;
                                self.state = NodeState::Follower;
                                self.res_append_entries(&msg)?;

                                break Ok(());
                            }
                        }

                        Message::AppendEntriesRes { term, .. } => {
                            if term > self.term {
                                self.term = term;
                                self.state = NodeState::Follower;

                                break Ok(());
                            }
                        }
                    }
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Election timeout.
                    // Start new election.
                    break Ok(());
                }

                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(Error::RxDisconnected);
                }
            }
        }
    }

    fn tick_leader(&mut self) -> Result<(), Error> {
        // TODO

        loop {
            // Sends AppendEntries to all nodes
            for peer in self.peers.values() {
                if let Err(mpsc::SendError(_)) = peer.send(Message::AppendEntries(AppendEntries {
                    term: self.term,
                    leader_id: self.id,
                    prev_log_id: self.log.last().map(|l| l.id),
                    entries: None, // TODO
                    leader_commit: self.commit_idx,
                })) {
                    return Err(Error::TxDisconnected);
                }
            }

            let deadline = Instant::now() + Duration::from_millis(self.heartbeat_tick);

            match self.recv_deadline(deadline) {
                Ok(msg) => match msg {
                    Message::RequestVote(msg) => {
                        if msg.term > self.term {
                            self.term = msg.term;
                            self.state = NodeState::Follower;
                            self.res_request_vote(&msg)?;
                            break Ok(());
                        }
                    }
                    Message::RequestVoteRes { term, .. } => {
                        if term > self.term {
                            self.term = term;
                            self.state = NodeState::Follower;
                            break Ok(());
                        }
                    }
                    Message::AppendEntries(msg) => {
                        if msg.term > self.term {
                            self.term = msg.term;
                            self.state = NodeState::Follower;
                            self.res_append_entries(&msg)?;
                            break Ok(());
                        }
                    }
                    Message::AppendEntriesRes { term, success } => {
                        if term > self.term {
                            self.term = term;
                            self.state = NodeState::Follower;
                            break Ok(());
                        }

                        if success {
                            // TODO
                        }
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Next AppendEntries
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(Error::RxDisconnected);
                }
            }
        }
    }

    // Responds to RequestVote.
    // Assumes it's a follower and `self.term >= (term of msg)`.
    fn res_request_vote(&mut self, msg: &RequestVote) -> Result<(), Error> {
        let RequestVote {
            term,
            candidate_id,
            latest_log_id,
        } = msg;

        let vote_granted = if term < &self.term {
            false
        } else {
            if self.voted_for.is_none() || self.is_log_up_to_date(latest_log_id) {
                self.voted_for = Some(*candidate_id);
                true
            } else {
                false
            }
        };

        let response = Message::RequestVoteRes {
            term: self.term,
            vote_granted,
        };

        self.peers
            .get(&candidate_id)
            .ok_or(Error::InvalidClusterConfig)?
            .send(response)
            .unwrap();

        Ok(())
    }

    // Responds to AppendEntries.
    // Assumes it's a follower and `self.term >= (term of msg)`.
    fn res_append_entries(&mut self, msg: &AppendEntries<T>) -> Result<(), Error> {
        let AppendEntries {
            term,
            leader_id,
            prev_log_id,
            entries,
            leader_commit,
        } = msg;

        let success = if term < &self.term || !self.contains_log_id(prev_log_id) {
            false
        } else {
            if let Some(entries) = entries {
                for entry in entries {
                    let id = entry.id;

                    if self.log.len() < id.idx && self.log[id.idx].id.term != id.term {
                        self.log.truncate(entry.id.idx);
                        break;
                    }
                }

                for entry in entries {
                    let id = entry.id;

                    if id.idx < self.log.len() {
                        self.log[id.idx] = entry.clone();
                    } else {
                        self.log.push(entry.clone());
                    }
                }
            }

            true
        };

        if leader_commit > &self.commit_idx {
            self.commit_idx = cmp::min(
                *leader_commit,
                self.log.last().map(|l| l.id.idx).unwrap_or(0),
            );
        }

        let response = Message::AppendEntriesRes {
            term: self.term,
            success,
        };

        self.peers
            .get(&leader_id)
            .ok_or(Error::InvalidClusterConfig)?
            .send(response)
            .unwrap();

        Ok(())
    }

    fn election_timeout(&self) -> Millisec {
        let (min, max) = (self.election_timeout_range.0, self.election_timeout_range.1);
        rand::thread_rng().gen_range(min, max)
    }

    fn majority(&self) -> usize {
        self.peers.len() / 2 + 1
    }

    // Returns whether `rhs` is more up-to-date than `self.log`.
    fn is_log_up_to_date(&self, rhs: &Option<LogEntryId>) -> bool {
        match (self.log.is_empty(), rhs) {
            (true, _) => true,
            (false, None) => false,
            (false, &Some(rhs)) => rhs >= self.log.last().unwrap().id,
        }
    }

    fn contains_log_id(&self, log_id: &Option<LogEntryId>) -> bool {
        if let Some(log_id) = log_id {
            self.log.len() > log_id.idx && self.log[log_id.idx].id.term == log_id.term
        } else {
            true
        }
    }
}

impl<T: Clone, R: Remote<T>> Local<T> for LocalNode<T, R> {
    fn try_recv(&self) -> Result<Message<T>, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    fn recv_deadline(&self, deadline: Instant) -> Result<Message<T>, mpsc::RecvTimeoutError> {
        self.rx.recv_deadline(deadline)
    }
}

/// State of each node
#[derive(Clone, Debug)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}
