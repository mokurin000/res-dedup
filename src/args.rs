use std::path::PathBuf;

#[derive(palc::Parser)]
pub struct Args {
    /// Concurrent limitation of parallel read
    #[arg(short, long, default_value_t = 64)]
    pub concurrency: usize,
    /// Paths to scan
    pub directories: Vec<PathBuf>,
}
