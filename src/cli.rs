use arg::Args;

#[derive(Args, Debug)]
///Utility to parser gcloud logs dump
pub struct Cli {
    #[arg(long, default)]
    ///Specifies to only count log entries number.
    pub count_only: bool,
    ///Path to the json log file.
    pub log: Vec<String>,
}

#[inline(always)]
pub fn args() -> Cli {
    arg::parse_args()
}
