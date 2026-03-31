//! Hecate user CLI entrypoint (implementation in later stories).

fn main() {
    assert!(hecate_core::workspace_ok(), "core crate should link");
    println!("hecate {}", env!("CARGO_PKG_VERSION"));
}
