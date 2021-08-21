use std::time::Duration;

pub const RETRY: u8 = 10;
pub const INIT_CONN_TIMEOUT: Duration = Duration::from_secs(10);
pub const ALIVE_TIMEOUT: Duration = Duration::from_secs(1);
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
pub const LEADER_CHECK_INTERVAL: Duration = Duration::from_secs(3);
