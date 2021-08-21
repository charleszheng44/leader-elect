use std::error::Error;

pub type ThreadSafeResult<T> = Result<T, Box<dyn Error + Sync + Send>>;

#[derive(Debug)]
pub struct LeaderElectError(String);

impl std::fmt::Display for LeaderElectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaderElectError(e) => write!(f, "{}", e),
        }
    }
}

impl LeaderElectError {
    pub fn new(err_str: String) -> LeaderElectError {
        LeaderElectError(err_str)
    }
}

impl Error for LeaderElectError {}

macro_rules! new_box_err {
    ($err_str:expr) => {
        Box::new(LeaderElectError::new($err_str))
    };
}
