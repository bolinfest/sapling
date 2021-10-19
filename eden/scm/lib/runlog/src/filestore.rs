/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Error;
use anyhow::Result;
use fs2::FileExt;
use util::lock::PathLock;
use util::path::create_shared_dir;

use crate::Entry;

pub struct FileStore {
    // Directory to write files to.
    dir: PathBuf,

    // A lock file indicating we are still running.
    #[allow(dead_code)]
    lock_file: PathLock,
}

const LOCK_EXT: &str = "lock";
const JSON_EXT: &str = "json";

/// FileStore is a simple runlog storage that writes JSON entries to a
/// specified directory.
impl FileStore {
    // Create a new FileStore that writes files to directory dir. dir
    // is created automatically if it doesn't exist.
    pub(crate) fn new(dir: PathBuf, entry_id: &str) -> Result<Self> {
        create_shared_dir(&dir)?;

        let lock_file = PathLock::exclusive(dir.join(entry_id).with_extension(LOCK_EXT))?;

        Ok(FileStore { dir, lock_file })
    }

    pub(crate) fn save(&self, e: &Entry) -> Result<()> {
        // Retry a few times since renaming file fails on windows if
        // destination path exists and is open.
        let mut retries = 3;
        loop {
            let res = self.save_attempt(e);
            if retries == 0 || res.is_ok() {
                break res;
            }
            retries -= 1;
            sleep(Duration::from_millis(5));
        }
    }

    fn save_attempt(&self, e: &Entry) -> Result<()> {
        // Write to temp file and rename to avoid incomplete writes.
        let tmp = tempfile::NamedTempFile::new_in(&self.dir)?;

        serde_json::to_writer_pretty(&tmp, e)?;

        // NB: we don't fsync so incomplete or empty JSON files are possible.

        tmp.persist(self.dir.join(&e.id).with_extension(JSON_EXT))?;

        Ok(())
    }

    pub(crate) fn cleanup<P: AsRef<Path>>(dir: P, threshold: Duration) -> Result<()> {
        create_shared_dir(&dir)?;

        for dir_entry in fs::read_dir(dir)? {
            let path = dir_entry?.path();

            let ext = match path.extension().and_then(OsStr::to_str) {
                Some(ext) => ext,
                _ => continue,
            };

            // Skip ".lock" files. This leaves ".json" and any stray tmp files.
            if ext == LOCK_EXT {
                continue;
            }

            // Command process is still running - don't clean up.
            if is_locked(&path)? {
                continue;
            }

            // Avoid trying to read the contents so we can clean up
            // incomplete files.
            let mtime = fs::metadata(&path)?.modified()?;
            if SystemTime::now().duration_since(mtime)? >= threshold {
                // Cleanup up ".json" or tmp file.
                remove_file_ignore_missing(&path)?;
                // Clean up lock file (we know command process isn't running anymore).
                remove_file_ignore_missing(path.with_extension(LOCK_EXT))?;
            }
        }

        Ok(())
    }

    // Iterates each entry, yielding the entry and whether the
    // associated command is still running.
    pub fn entry_iter<P: AsRef<Path>>(
        dir: P,
    ) -> Result<impl Iterator<Item = Result<(Entry, bool), Error>>> {
        create_shared_dir(&dir)?;

        Ok(fs::read_dir(&dir)?.filter_map(|file| match file {
            Ok(file) => {
                // We only care about ".json" files.
                match file.path().extension().and_then(OsStr::to_str) {
                    Some(ext) if ext == JSON_EXT => {}
                    _ => return None,
                };

                match fs::File::open(file.path()) {
                    Ok(f) => Some(
                        serde_json::from_reader(&f)
                            .map_err(Error::new)
                            .and_then(|e| Ok((e, is_locked(file.path())?))),
                    ),
                    Err(err) => Some(Err(Error::new(err))),
                }
            }
            Err(err) => Some(Err(Error::new(err))),
        }))
    }
}

fn remove_file_ignore_missing<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::remove_file(&path).or_else(|err| match err.kind() {
        io::ErrorKind::NotFound => Ok(()),
        _ => Err(err),
    })
}

// Return whether path's corresponding locked file is exclusively
// locked (by running command). Return false if lock file doesn't exist.
fn is_locked<P: AsRef<Path>>(path: P) -> Result<bool> {
    match fs::File::open(path.as_ref().with_extension(LOCK_EXT)) {
        Ok(f) => Ok(f.try_lock_shared().is_err()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(Error::new(err)),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_save() {
        let td = tempdir().unwrap();

        let fs_dir = td.path().join("banana");
        let fs = FileStore::new(fs_dir.clone(), "some_id").unwrap();
        // Make sure FileStore creates directory automatically.
        assert!(fs_dir.exists());

        let mut entry = Entry::new(vec!["some_command".to_string()]);

        let assert_entry = |e: &Entry| {
            let f = fs::File::open(fs_dir.join(&e.id).with_extension(JSON_EXT)).unwrap();
            let got: Entry = serde_json::from_reader(&f).unwrap();
            assert_eq!(&got, e);
        };

        // Can create new entry.
        fs.save(&entry).unwrap();
        assert_entry(&entry);

        // Can update existing entry.
        entry.pid = 1234;
        fs.save(&entry).unwrap();
        assert_entry(&entry);
    }

    #[test]
    fn test_cleanup() {
        let td = tempdir().unwrap();
        let e = Entry::new(vec!["foo".to_string()]);
        let entry_path = td.path().join(&e.id).with_extension(JSON_EXT);

        {
            let fs = FileStore::new(td.path().into(), &e.id).unwrap();
            fs.save(&e).unwrap();

            // Still locked, don't clean up.
            FileStore::cleanup(&td, Duration::ZERO).unwrap();
            assert!(entry_path.exists());
        }

        // No longer locked since file store is closed, but haven't met threshold.
        FileStore::cleanup(&td, Duration::from_secs(3600)).unwrap();
        assert!(entry_path.exists());

        // Met threshold - delete.
        FileStore::cleanup(&td, Duration::ZERO).unwrap();
        assert!(!entry_path.exists());
    }

    #[test]
    fn test_iter() {
        let td = tempdir().unwrap();

        let a = Entry::new(vec!["a".to_string()]);
        let a_fs = FileStore::new(td.path().into(), &a.id).unwrap();
        a_fs.save(&a).unwrap();


        let b = Entry::new(vec!["b".to_string()]);
        {
            let b_fs = FileStore::new(td.path().into(), &b.id).unwrap();
            b_fs.save(&b).unwrap();
        }

        let mut got: Vec<(Entry, bool)> = FileStore::entry_iter(td.path())
            .unwrap()
            .map(Result::unwrap)
            .collect();
        got.sort_by(|a, b| a.0.command[0].cmp(&b.0.command[0]));

        assert_eq!(vec![(a, true), (b, false)], got)
    }
}
