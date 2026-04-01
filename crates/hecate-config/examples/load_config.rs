//! Inspect merged config the same way the app will (real `dirs::config_dir`, real env).
//!
//! From the repo root:
//!
//! ```text
//! cargo run -p hecate-config --example load_config
//! cargo run -p hecate-config --example load_config -- /path/to/git/repo
//! HECATE_ROOT=/tmp/hecate-data cargo run -p hecate-config --example load_config
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    let opts = hecate_config::LoadOptions {
        repo_root,
        config_home_override: None,
    };

    match hecate_config::load(&opts) {
        Ok(cfg) => {
            println!("{cfg:#?}");
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
