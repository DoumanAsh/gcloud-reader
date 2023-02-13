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
impl Cli {
    #[inline]
    pub fn new<'a, T: IntoIterator<Item = &'a str>>(args: T) -> Result<Self, bool> {
        let args = args.into_iter();

        Cli::from_args(args).map_err(|err| match err.is_help() {
            true => {
                println!("{}", Cli::HELP);
                false
            },
            false => {
                eprintln!("{}", err);
                true
            },
        })
    }
}
