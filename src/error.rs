#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "invalid cluster config")]
    InvalidClusterConfig,
}
