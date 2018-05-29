use Term;
use log::{LogEntry, LogEntryId};

#[derive(Debug)]
pub enum Message<T> {
    RequestVote {
        term: Term,
        latest_log_id: LogEntryId,
    },
    AppendEntry {
        new_log: LogEntry<T>,
        last_log_id: LogEntryId,
    },
    Reject,
}
