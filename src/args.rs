use std::{path::PathBuf, str::FromStr};

#[derive(palc::Parser)]
pub struct Args {
    /// Concurrent limitation of parallel read
    #[arg(short, long, default_value_t = 64)]
    pub concurrency: usize,
    /// Buffer size
    #[arg(short = 'B', long, default_value = "256 KiB")]
    pub buf_size: Size,
    /// Paths to scan
    pub directories: Vec<PathBuf>,
}

impl FromStr for Size {
    type Err = parse_size::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Size(parse_size::parse_size(s)?))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]

pub struct Size(u64);
impl Size {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}
