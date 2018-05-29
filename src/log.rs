use Term;

#[derive(Clone, Debug)]
pub struct LogEntry<T: Clone> {
    pub id: LogEntryId,
    command: T,
}

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct LogEntryId {
    pub term: Term,
    pub idx: usize,
}
