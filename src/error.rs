#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "invalid node config")]
    InvalidNodeConfig,
}
