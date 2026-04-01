//! Print merged metadata for a `hecate_root` directory (creates nothing by default).
//!
//! ```text
//! cargo run -p hecate-config --example print_metadata -- /path/to/hecate_root
//! ```

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: print_metadata <hecate_root>")?;
    let meta = hecate_config::read_metadata(Path::new(&root))?;
    println!("{meta:#?}");
    Ok(())
}
