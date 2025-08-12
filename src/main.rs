use std::hash::Hasher;
use std::{io, path::PathBuf, time::SystemTime};

use arrayvec::ArrayVec;
use compio::BufResult;
use compio::fs::OpenOptions;
use compio::io::AsyncReadAt;
use compio::runtime::spawn_blocking;
use dashmap::DashMap;
use futures_util::StreamExt;
use palc::Parser;

use rapidhash::fast::{RandomState, RapidHashMap, RapidHasher};
use res_dedup_hardlink::{args::Args, scan::visit_dirs};

fn main() {
    let scan_time = SystemTime::now();
    let Args {
        directories,
        concurrency,
    } = Args::parse();

    // FileId - File Path
    // skip already hard linked files
    let mut fileid_path = RapidHashMap::default();
    for path in directories {
        _ = visit_dirs(path, &mut fileid_path);
    }
    println!(
        "found {} files in {}msec",
        fileid_path.len(),
        scan_time.elapsed().unwrap_or_default().as_millis()
    );

    let files = fileid_path.into_values();
    if let Err(e) = dedup_files(concurrency, files) {
        eprintln!("read error: {e}");
    }
}

#[compio::main]
async fn dedup_files(
    concurrency: usize,
    files: impl ExactSizeIterator + Iterator<Item = PathBuf>,
) -> Result<(), io::Error> {
    let hash_file: DashMap<u64, PathBuf, RandomState> = DashMap::with_hasher(RandomState::new());

    futures_util::stream::iter(files)
        .map(async |file_path| {
            let Ok(file) = OpenOptions::new().read(true).open(&file_path).await else {
                return;
            };

            let mut buf: ArrayVec<u8, { 16 * 1024 }> = ArrayVec::new_const();
            let mut pos = 0;
            let mut hasher = RapidHasher::default_const();

            loop {
                buf.clear();
                match file.read_at(buf, pos).await {
                    BufResult(Ok(0), _) => break,
                    BufResult(Ok(len), buf_) => {
                        let bytes = &buf_[..len];
                        pos += bytes.len() as u64;

                        if cfg!(feature = "spawn_thread") {
                            let bytes = bytes.to_vec();
                            hasher = spawn_blocking(move || {
                                hasher.write(&bytes);
                                hasher
                            })
                            .await
                            .expect("join error");
                        } else {
                            hasher.write(bytes);
                        }

                        buf = buf_;
                    }
                    BufResult(Err(e), buf_) => {
                        buf = buf_;
                        eprintln!("failed to read: {e}");
                    }
                }
            }

            let hash = hasher.finish();

            if let Some(existing) = hash_file.get(&hash) {
                println!(
                    "{} <=> {}",
                    existing.to_string_lossy(),
                    file_path.to_string_lossy()
                );
            } else {
                hash_file.insert(hash, file_path);
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .into_future()
        .await;

    Ok(())
}
