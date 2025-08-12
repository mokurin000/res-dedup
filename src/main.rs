use std::{
    fs::read_dir,
    io,
    path::{Path, PathBuf, absolute},
    time::SystemTime,
};

use palc::Parser;

use crate::args::Args;

mod args;

fn main() {
    let scan_time = SystemTime::now();
    let Args { directories } = Args::parse();
    let mut files = Vec::new();
    for path in directories {
        _ = fast_scan(&mut files, path);
    }
    println!(
        "found {} files in {}msec",
        files.len(),
        scan_time.elapsed().unwrap_or_default().as_millis()
    );
}

fn fast_scan(out: &mut Vec<PathBuf>, path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();

    let Ok(entries) = read_dir(path) else {
        eprintln!("{path:?} was not a directory, skipping!");
        return Ok(());
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if entry.metadata().is_ok_and(|m| m.is_dir()) {
            fast_scan(out, path)?;
        } else {
            out.push(absolute(path)?);
        }
    }

    Ok(())
}
