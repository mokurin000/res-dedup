use std::path::PathBuf;

#[derive(palc::Parser)]
pub struct Args {
    /// Paths to scan
    pub directories: Vec<PathBuf>,
}
