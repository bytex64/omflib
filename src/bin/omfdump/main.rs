use std::{fs, path::PathBuf, process::ExitCode};

use clap::Parser;
use omflib::OmfReader;

#[derive(Parser, Debug)]
struct Args {
    file: PathBuf,
}

pub fn main() -> ExitCode {
    let args = Args::parse();
    let mut f = fs::File::open(args.file).expect("Could not open input file");
    let reader = OmfReader::new(&mut f);
    for section in reader {
        println!("{}", section);
    }
    ExitCode::SUCCESS
}
