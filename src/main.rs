use clap::{Parser, Subcommand};
use decoder::{decode_flat_files, stream_blocks};
use std::io::{self, BufReader, BufWriter};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Stream data continuously
    Stream{
        #[clap(short, long, default_value = "false")]
        decompress: bool,
    },
    /// Decode files from input to output
    Decode {
        #[clap(short, long)]
        input: Option<String>,
        #[clap(long)]
        headers_dir: Option<String>,
        #[clap(short, long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stream { decompress } => {
            if decompress {
                // zst decompress first
                let reader = zstd::stream::Decoder::new(io::stdin()).expect("Failed to create zstd decoder");
                let writer = BufWriter::new(io::stdout().lock());
                stream_blocks(reader, writer).expect("Failed to stream blocks");
            }
            else {
                let reader = BufReader::with_capacity(64 * 2 << 20, io::stdin().lock());
                let writer = BufWriter::new(io::stdout().lock());
                stream_blocks(reader, writer).expect("Failed to stream blocks");
            }
            
        }
        Commands::Decode { input, headers_dir, output } => {
            let input = match input {
                Some(input) => decoder::DecodeInput::Path(input),
                None => decoder::DecodeInput::Reader(Box::new(BufReader::new(io::stdin()))),
            };

            let blocks = decode_flat_files(input, output.as_deref(), headers_dir.as_deref())
                .expect("Failed to decode files");

            println!("Total blocks: {}", blocks.len());
        }
    }
}
