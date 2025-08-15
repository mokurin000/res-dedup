use std::hash::Hasher;
use std::{io, path::PathBuf, time::SystemTime};

use compio::BufResult;
use compio::fs::OpenOptions;
use compio::io::AsyncReadAt;
use compio::runtime::spawn_blocking;
use dashmap::DashMap;
use futures_util::StreamExt;
use palc::Parser;

use rapidhash::fast::{RandomState, RapidHashMap, RapidHasher};
use res_dedup::{args::Args, scan::visit_dirs};

fn main() {
    let scan_time = SystemTime::now();
    let Args {
        directories,
        concurrency,
        buf_size,
    } = Args::parse();
    let buf_size = parse_size::parse_size(buf_size).unwrap_or(32 * 1024) as usize;

    // FileId - File Path
    // skip already hard linked files
    let mut fileid_path = RapidHashMap::default();
    for path in directories {
        _ = visit_dirs(path, &mut fileid_path);
    }
    eprintln!(
        "found {} files in {}msec",
        fileid_path.len(),
        scan_time.elapsed().unwrap_or_default().as_millis()
    );

    let files = fileid_path.into_values();
    if let Err(e) = dedup_files(concurrency, buf_size, files) {
        eprintln!("read error: {e}");
    }
}

#[compio::main]
async fn dedup_files(
    concurrency: usize,
    buf_size: usize,
    files: impl ExactSizeIterator + Iterator<Item = PathBuf>,
) -> Result<(), io::Error> {
    let hash_file: DashMap<u64, PathBuf, RandomState> = DashMap::with_hasher(RandomState::new());

    futures_util::stream::iter(files)
        .map(async |file_path| {
            let Ok(file) = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .custom_flags(
                    #[cfg(windows)]
                    {
                        windows::Win32::Storage::FileSystem::FILE_FLAG_SEQUENTIAL_SCAN.0
                    },
                    #[cfg(not(windows))]
                    0,
                )
                .open(&file_path)
                .await
            else {
                return;
            };

            let mut buf = Vec::with_capacity(buf_size);
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
                    BufResult(Err(e), _) => {
                        eprintln!("failed to read: {e}, pos: {pos}");
                        return;
                    }
                }
            }

            let hash = hasher.finish();

            if let Some(existing) = hash_file.get(&hash) {
                println!(
                    "{{\"source\": {:?}, \"other\": {file_path:?}}}",
                    existing.as_path(),
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
