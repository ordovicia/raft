use Term;

pub struct LogEntry<T> {
    id: LogEntryId,
    command: T,
}

pub struct LogEntryId {
    term: Term,
    idx: usize,
}
