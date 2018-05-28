use std::sync::mpsc;
use std::time::Duration;

pub trait Recv<T> {
    fn try_recv(&self) -> Result<T, mpsc::TryRecvError>;
    fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError>;
}

impl<T> Recv<T> for mpsc::Receiver<T> {
    fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.try_recv()
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError> {
        self.recv_timeout(timeout)
    }
}

pub trait Send<T> {
    fn send(&self, t: T) -> Result<(), mpsc::SendError<T>>;
}

impl<T> Send<T> for mpsc::Sender<T> {
    fn send(&self, t: T) -> Result<(), mpsc::SendError<T>> {
        self.send(t)
    }
}
