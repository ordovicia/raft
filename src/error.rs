#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "invalid cluster config")]
    InvalidClusterConfig,

    #[fail(display = "RX disconnected")]
    RxDisconnected,

    #[fail(display = "TX disconnected")]
    TxDisconnected,
}
