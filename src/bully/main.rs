use clap::Clap;
use leader_elect::bully::{run, Opts};
use leader_elect::error::ThreadSafeResult;
use leader_elect::logger;

fn main() -> ThreadSafeResult<()> {
    let opts: Opts = Opts::parse();
    logger::init(opts.log_level.as_ref()).expect("fail to set the logger");
    run(&opts)
}
