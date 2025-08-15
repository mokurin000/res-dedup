use std::path::{Path, PathBuf, absolute};

use core::slice;
use std::{
    error::Error,
    ffi::OsString,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{FromRawHandle, OwnedHandle},
    },
};

use rapidhash::fast::RapidHashMap;
use windows::{
    Win32::{
        Foundation::GENERIC_READ,
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAGS_AND_ATTRIBUTES, FILE_ID_BOTH_DIR_INFO,
            FILE_SHARE_READ, FileIdBothDirectoryInfo, GetFileInformationByHandleEx, OPEN_EXISTING,
        },
    },
    core::PCWSTR,
};

pub fn visit_dirs(
    dirpath: impl AsRef<Path>,
    out: &mut RapidHashMap<i64, PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let path = dirpath.as_ref();

    unsafe {
        let hfile = {
            let mut raw_path = path.as_os_str().encode_wide().collect::<Vec<u16>>();
            raw_path.push(0);
            let path = PCWSTR::from_raw(raw_path.as_ptr());
            CreateFileW(
                path,
                GENERIC_READ.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }?;
        let _drop = OwnedHandle::from_raw_handle(hfile.0);
        loop {
            let mut buf = [0_u8; 32768];
            let Ok(_) = GetFileInformationByHandleEx(
                hfile,
                FileIdBothDirectoryInfo,
                buf.as_mut_ptr() as _,
                buf.len() as u32,
            ) else {
                break Ok(());
            };

            let mut start = 0;

            loop {
                let last_ptr = get_info(path, &buf, &mut start, out)?;
                if (&*last_ptr).NextEntryOffset == 0 {
                    break;
                }
            }
        }
    }
}

unsafe fn get_info(
    path: &Path,
    buf: &[u8],
    start: &mut usize,
    out: &mut RapidHashMap<i64, PathBuf>,
) -> Result<*const FILE_ID_BOTH_DIR_INFO, Box<dyn Error>> {
    unsafe {
        let raw_ptr: *const FILE_ID_BOTH_DIR_INFO = buf.as_ptr().add(*start) as _;
        let info: &FILE_ID_BOTH_DIR_INFO = &*raw_ptr;
        *start += info.NextEntryOffset as usize;

        let file_name_slice =
            slice::from_raw_parts(info.FileName.as_ptr(), (info.FileNameLength / 2) as _);

        let file_name = OsString::from_wide(file_name_slice);
        let file_path = path.join(&file_name);

        if [".", ".."].contains(&file_name.to_string_lossy().as_ref()) {
            return Ok(raw_ptr);
        }

        if FILE_FLAGS_AND_ATTRIBUTES(info.FileAttributes).contains(FILE_ATTRIBUTE_DIRECTORY) {
            visit_dirs(&file_path, out)?;

            return Ok(raw_ptr);
        }

        let inode = info.FileId;
        out.insert(inode, absolute(file_path)?);

        Ok(raw_ptr)
    }
}
