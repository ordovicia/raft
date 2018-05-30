use Term;
use log::{LogEntry, LogEntryId};

#[derive(Debug)]
pub enum Message<T: Clone> {
    // RequestVote
    RequestVote(RequestVote),

    RequestVoteRes {
        /// Term, for candidate to update itself
        term: Term,
        /// True means candidate received vote
        vote_granted: bool,
    },

    // AppendEntries
    AppendEntries(AppendEntries<T>),

    AppendEntriesRes {
        /// Term, for leader to update itself
        term: Term,
        /// True if follower contained entry matching `prev_log_id`
        success: bool,
    },
}

#[derive(Debug)]
pub struct RequestVote {
    /// Candidate's term
    pub term: Term,
    /// Candidate requesting vote
    pub candidate_id: usize,
    /// ID of candidate's last log entry
    pub latest_log_id: Option<LogEntryId>,
}

#[derive(Debug)]
pub struct AppendEntries<T: Clone> {
    /// Leader's term
    pub term: Term,

    /// So follower can redirect clients
    pub leader_id: usize,

    /// ID of log entry immediately preceding new ones
    pub prev_log_id: Option<LogEntryId>,

    /// Log entries to store (`None` for heatbeat).
    /// Assuming ordered by older-to-newer.
    pub entries: Option<Vec<LogEntry<T>>>,

    /// Leader's `commit_idx`
    pub leader_commit: usize,
}
