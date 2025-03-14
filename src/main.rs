use std::process::ExitCode;

use gcloud_reader::cli::args;

fn main() -> ExitCode {
    let args = args();

    let mut count = 0;
    for path in args.log {
        let reader = match gcloud_reader::read_file(&path) {
            Ok(reader) => reader,
            Err(error) => {
                eprintln!("{path}: Error reading file: {error}");
                continue;
            }
        };


        for (idx, value) in reader.enumerate() {
            let entry = match value {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("record(idx={idx}) error: {error}");
                    return ExitCode::FAILURE;
                }
            };
            if !args.count_only {
                println!("[{}] {}", entry.timestamp, entry.text_payload);
            }
            count += 1;
        }
    }

    println!("Log entries count: {count}");

    ExitCode::SUCCESS
}
