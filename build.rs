use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["src/protos/block.proto"], &["src/"])?;
    prost_build::compile_protos(&["src/protos/bstream.proto"], &["src/"])?;
    Ok(())
}
