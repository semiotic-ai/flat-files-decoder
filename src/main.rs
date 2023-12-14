use std::io::BufReader;

use clap::Parser;
use decoder::{decode_flat_files, stream_blocks};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    input: Option<String>,
    #[clap(long)]
    headers_dir: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();
    // let input = match args.input {
    //     Some(input) => decoder::DecodeInput::Path(input),
    //     None => decoder::DecodeInput::Reader(Box::new(std::io::stdin())),
    // };

    let reader = BufReader::with_capacity(64 * 2<<20, std::io::stdin().lock());
    let result = stream_blocks(reader, std::io::stdout().lock());
    // let blocks = decode_flat_files(input, args.output.as_deref(), args.headers_dir.as_deref())
    //     .expect("Failed to decode files");

    // println!("Total blocks: {}", blocks.len());
}
