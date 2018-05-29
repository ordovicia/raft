use Term;

#[derive(Debug)]
pub struct LogEntry<T> {
    id: LogEntryId,
    command: T,
}

#[derive(Debug)]
pub struct LogEntryId {
    term: Term,
    idx: usize,
}
