# res-dedup

[FileId]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-by_handle_file_information#remarks

Fast file-level duplication scanner for Windows, with [FileId] awareness.

> This tool was mainly designed for \(and benchmarked on\) SSD. Parallel sequential read streams are impossible for HDD.

[^0]: https://learn.microsoft.com/en-us/windows/win32/api/WinBase/nf-winbase-createhardlinka#remarks

## Benchmark

- OS: Windows 11 24H2 / Revision patched
- CPU: i7-12700H
- SSD: Acer GM7000
- Cache Size: 256 KiB
- Concurrency: 128
- Benchmark tool: perfmon.exe

| Item      | Average    | Peak       |
| :-------- | :--------- | :--------- |
| IO Read   | 3839 MiB/s | 3951 MiB/s |
| Read op/s | 22162.8    | 25433.4    |


## TODO

- Linux support (with getdents64)

## Usage

See `-h` / `--help`.

## Output format

[JSON Lines]: https://jsonlines.org/

[JSON Lines] of {"source": string, "other": string}

`source` is the path to first file found with the same hash with `other`.

## Purpose

Reasonbly fast & lightweight file duplication scanner.

By design, hard links are treated as non-duplication.

## Non-purpose

Replacement of general & powerful deduplication tools like [jdupes](https://codeberg.org/jbruchon/jdupes)

## Bahaviour

> When you create a hard link on the NTFS file system,
> 
> the file attribute information in the directory entry is refreshed only when the file is opened,
> 
> or when GetFileInformationByHandle is called with the handle of a specific file. [^0]

Thus, this project would never care about file attributes, but the file content itself.

Media similarity is also out of scope.
