use clap::Parser;
use decoder::{decode_flat_files, stream_blocks};
use std::io::{self, BufReader, BufWriter};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    input: Option<String>,
    #[clap(long)]
    headers_dir: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
    #[clap(long)]
    stream: bool, // Add a flag for streaming
}

fn main() {
    let args = Args::parse();

    // Check if the stream flag is set
    if args.stream {
        let reader = BufReader::with_capacity(64 * 2 << 20, io::stdin().lock());
        let writer = BufWriter::new(io::stdout().lock());
        stream_blocks(reader, writer).expect("Failed to stream blocks");
    } else {
        // Use file paths or stdin/out as before
        let input = match args.input {
            Some(input) => decoder::DecodeInput::Path(input),
            None => decoder::DecodeInput::Reader(Box::new(BufReader::new(io::stdin()))),
        };

        let blocks = decode_flat_files(input, args.output.as_deref(), args.headers_dir.as_deref())
            .expect("Failed to decode files");

        println!("Total blocks: {}", blocks.len());
    }
}
