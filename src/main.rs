#![no_main]

use gcloud_reader::cli::Cli;

c_ffi::c_main!(rust_main);

fn rust_main(args: c_ffi::Args) -> bool {
    let args = match Cli::new(args.into_iter().skip(1)) {
        Ok(args) => args,
        Err(code) => return code,
    };

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
                    return false;
                }
            };
            if !args.count_only {
                println!("{}", entry.text_payload);
            }
            count += 1;
        }
    }

    println!("Log entries count: {count}");

    true
}
