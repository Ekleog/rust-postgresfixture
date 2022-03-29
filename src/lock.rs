use std::fs::File;
use std::os::unix::io::AsRawFd;

use nix::fcntl::{flock, FlockArg};
use nix::Result;

#[derive(Debug)]
pub(crate) struct UnlockedFile(File);
#[derive(Debug)]
pub(crate) struct LockedFileShared(File);
#[derive(Debug)]
pub(crate) struct LockedFileExclusive(File);

#[allow(unused)]
impl UnlockedFile {
    /// Create a new `UnlockedFile` for the given `File`.
    pub fn from(file: File) -> Self {
        Self(file)
    }

    pub fn try_lock_shared(self) -> Result<LockedFileShared> {
        flock(self.0.as_raw_fd(), FlockArg::LockSharedNonblock)?;
        Ok(LockedFileShared(self.0))
    }

    pub fn lock_shared(self) -> Result<LockedFileShared> {
        flock(self.0.as_raw_fd(), FlockArg::LockShared)?;
        Ok(LockedFileShared(self.0))
    }

    pub fn try_lock_exclusive(self) -> Result<LockedFileExclusive> {
        flock(self.0.as_raw_fd(), FlockArg::LockExclusiveNonblock)?;
        Ok(LockedFileExclusive(self.0))
    }

    pub fn lock_exclusive(self) -> Result<LockedFileExclusive> {
        flock(self.0.as_raw_fd(), FlockArg::LockExclusive)?;
        Ok(LockedFileExclusive(self.0))
    }
}

#[allow(unused)]
impl LockedFileShared {
    pub fn try_lock_exclusive(self) -> Result<LockedFileExclusive> {
        flock(self.0.as_raw_fd(), FlockArg::LockExclusiveNonblock)?;
        Ok(LockedFileExclusive(self.0))
    }

    pub fn lock_exclusive(self) -> Result<LockedFileExclusive> {
        flock(self.0.as_raw_fd(), FlockArg::LockExclusive)?;
        Ok(LockedFileExclusive(self.0))
    }

    pub fn try_unlock(self) -> Result<UnlockedFile> {
        flock(self.0.as_raw_fd(), FlockArg::UnlockNonblock)?;
        Ok(UnlockedFile(self.0))
    }

    pub fn unlock(self) -> Result<UnlockedFile> {
        flock(self.0.as_raw_fd(), FlockArg::Unlock)?;
        Ok(UnlockedFile(self.0))
    }
}

#[allow(unused)]
impl LockedFileExclusive {
    pub fn try_lock_shared(self) -> Result<LockedFileShared> {
        flock(self.0.as_raw_fd(), FlockArg::LockSharedNonblock)?;
        Ok(LockedFileShared(self.0))
    }

    pub fn lock_shared(self) -> Result<LockedFileShared> {
        flock(self.0.as_raw_fd(), FlockArg::LockShared)?;
        Ok(LockedFileShared(self.0))
    }

    pub fn try_unlock(self) -> Result<UnlockedFile> {
        flock(self.0.as_raw_fd(), FlockArg::UnlockNonblock)?;
        Ok(UnlockedFile(self.0))
    }

    pub fn unlock(self) -> Result<UnlockedFile> {
        flock(self.0.as_raw_fd(), FlockArg::Unlock)?;
        Ok(UnlockedFile(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::UnlockedFile;

    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;
    use std::path::Path;

    use nix::fcntl::{flock, FlockArg};

    fn can_lock<P: AsRef<Path>>(filename: P, exclusive: bool) -> bool {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(filename)
            .unwrap();
        let mode = match exclusive {
            true => FlockArg::LockExclusiveNonblock,
            false => FlockArg::LockSharedNonblock,
        };
        flock(file.as_raw_fd(), mode).is_ok()
    }

    fn can_lock_exclusive<P: AsRef<Path>>(filename: P) -> bool {
        can_lock(filename, true)
    }

    fn can_lock_shared<P: AsRef<Path>>(filename: P) -> bool {
        can_lock(filename, false)
    }

    #[test]
    fn file_lock_exclusive_takes_exclusive_flock() {
        let lock_dir = tempdir::TempDir::new("locks").unwrap();
        let lock_filename = lock_dir.path().join("lock");
        let lock = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&lock_filename)
            .map(UnlockedFile::from)
            .unwrap();

        assert!(can_lock_exclusive(&lock_filename));
        assert!(can_lock_shared(&lock_filename));

        let lock = lock.lock_exclusive().unwrap();

        assert!(!can_lock_exclusive(&lock_filename));
        assert!(!can_lock_shared(&lock_filename));

        lock.unlock().unwrap();

        assert!(can_lock_exclusive(&lock_filename));
        assert!(can_lock_shared(&lock_filename));
    }

    #[test]
    fn file_try_lock_exclusive_does_not_block_on_existing_shared_lock() {
        let lock_dir = tempdir::TempDir::new("locks").unwrap();
        let lock_filename = lock_dir.path().join("lock");
        let open_lock_file = || {
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&lock_filename)
                .map(UnlockedFile::from)
                .unwrap()
        };

        let _lock_shared = open_lock_file().lock_shared().unwrap();

        assert!(match open_lock_file().try_lock_exclusive() {
            Err(nix::errno::Errno::EAGAIN) => true,
            _ => false,
        });
    }

    #[test]
    fn file_try_lock_exclusive_does_not_block_on_existing_exclusive_lock() {
        let lock_dir = tempdir::TempDir::new("locks").unwrap();
        let lock_filename = lock_dir.path().join("lock");
        let open_lock_file = || {
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&lock_filename)
                .map(UnlockedFile::from)
                .unwrap()
        };

        let _lock_exclusive = open_lock_file().lock_exclusive().unwrap();

        assert!(match open_lock_file().try_lock_exclusive() {
            Err(nix::errno::Errno::EAGAIN) => true,
            _ => false,
        });
    }
}
