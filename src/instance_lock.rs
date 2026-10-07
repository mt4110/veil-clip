//! macOSの協調的なプロセス排他。クリップボードには接続しません。
//! Cooperative macOS process locking, with no clipboard access.

use std::fs::File;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use rustix::fd::OwnedFd;
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, Stat};
use rustix::io::Errno;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockError {
    UserDirectory,
    UnsafePath,
    Occupied,
    Access,
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UserDirectory => "ユーザー保存先を取得できません / User directory unavailable",
            Self::UnsafePath => "保存先の安全条件を満たしません / Unsafe lock path",
            Self::Occupied => "別プロセスがロックを保持しています / Lock already held",
            Self::Access => "ロックを取得できません / Lock access failed",
        })
    }
}

impl std::error::Error for LockError {}

// Cloneやハンドルの公開を行わず、Dropによるcloseまでロックを保持する。
// No clone or exposed handles; closing the owned File releases the lock.
pub struct InstanceLock {
    _file: File,
    _directory: OwnedFd,
}

impl InstanceLock {
    pub fn acquire() -> Result<Self, LockError> {
        let output = Command::new("/usr/bin/getconf")
            .arg("DARWIN_USER_DIR")
            .env_clear()
            .output()
            .map_err(|_| LockError::UserDirectory)?;
        if !output.status.success() {
            return Err(LockError::UserDirectory);
        }
        let path = user_path(&output.stdout)?;
        Self::acquire_in(&path)
    }

    pub(crate) fn acquire_in(base: &Path) -> Result<Self, LockError> {
        let uid = rustix::process::getuid();
        if uid != rustix::process::geteuid() || uid.is_root() {
            return Err(LockError::UnsafePath);
        }
        let uid = uid.as_raw();
        let base = open_directory_chain(base, uid)?;
        let base_stat = fs::fstat(&base).map_err(|_| LockError::Access)?;
        // OS管理の親は0755でもよい。書込権限は所有者だけに限定する。
        // OS-managed parents may be 0755; only the owner may write.
        if base_stat.st_uid != uid {
            return Err(LockError::UnsafePath);
        }
        match fs::mkdirat(&base, "veil-clip", Mode::from_raw_mode(0o700)) {
            Ok(()) | Err(Errno::EXIST) => {}
            Err(_) => return Err(LockError::Access),
        }
        let directory = fs::openat(&base, "veil-clip", directory_flags(), Mode::empty())
            .map_err(|_| LockError::UnsafePath)?;
        validate_private_directory(&fs::fstat(&directory).map_err(|_| LockError::Access)?, uid)?;
        let fd = fs::openat(
            &directory,
            "apply.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| LockError::UnsafePath)?;
        let file = File::from(fd);
        validate_lock_file(&fs::fstat(&file).map_err(|_| LockError::Access)?, uid)?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => LockError::Occupied,
            std::fs::TryLockError::Error(_) => LockError::Access,
        })?;
        // 開いた実体がパス上の実体と同じか、取得後にも確認する。
        // Check that the acquired inode still matches its directory entry.
        same_entry(&base, "veil-clip", &directory)?;
        same_entry(&directory, "apply.lock", &file)?;
        validate_private_directory(&fs::fstat(&directory).map_err(|_| LockError::Access)?, uid)?;
        validate_lock_file(&fs::fstat(&file).map_err(|_| LockError::Access)?, uid)?;
        Ok(Self {
            _file: file,
            _directory: directory,
        })
    }
}

fn user_path(output: &[u8]) -> Result<PathBuf, LockError> {
    let value = std::str::from_utf8(output).map_err(|_| LockError::UserDirectory)?;
    let value = value.strip_suffix('\n').unwrap_or(value);
    // /varはmacOSの既知のリンク。その他のリンクは開く際に拒否する。
    // Normalize only the known macOS /var alias; reject other links on open.
    let path = Path::new(value);
    let suffix = path
        .strip_prefix("/var/folders")
        .or_else(|_| path.strip_prefix("/private/var/folders"))
        .map_err(|_| LockError::UnsafePath)?;
    if suffix.as_os_str().is_empty()
        || !suffix
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(LockError::UnsafePath);
    }
    Ok(Path::new("/private/var/folders").join(suffix))
}

fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn open_directory_chain(path: &Path, uid: u32) -> Result<OwnedFd, LockError> {
    if !path.is_absolute() {
        return Err(LockError::UnsafePath);
    }
    let mut directory =
        fs::open("/", directory_flags(), Mode::empty()).map_err(|_| LockError::Access)?;
    for component in path.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => {
                directory = fs::openat(&directory, name, directory_flags(), Mode::empty())
                    .map_err(|_| LockError::UnsafePath)?;
            }
            _ => return Err(LockError::UnsafePath),
        }
        let stat = fs::fstat(&directory).map_err(|_| LockError::Access)?;
        if (stat.st_uid != 0 && stat.st_uid != uid) || stat.st_mode & 0o022 != 0 {
            return Err(LockError::UnsafePath);
        }
    }
    Ok(directory)
}

fn validate_private_directory(stat: &Stat, uid: u32) -> Result<(), LockError> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_uid != uid
        || stat.st_mode & 0o7777 != 0o700
    {
        return Err(LockError::UnsafePath);
    }
    Ok(())
}

fn validate_lock_file(stat: &Stat, uid: u32) -> Result<(), LockError> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != uid
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_nlink != 1
    {
        return Err(LockError::UnsafePath);
    }
    Ok(())
}

fn same_entry(
    parent: &impl rustix::fd::AsFd,
    name: &str,
    opened: &impl rustix::fd::AsFd,
) -> Result<(), LockError> {
    let entry =
        fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| LockError::UnsafePath)?;
    let held = fs::fstat(opened).map_err(|_| LockError::Access)?;
    if entry.st_dev != held.st_dev || entry.st_ino != held.st_ino {
        return Err(LockError::UnsafePath);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_untrusted_user_directory_output() {
        for output in [
            b"".as_slice(),
            b"/tmp/test\n",
            b"/var/folders/../test\n",
            b"/var/folders\n",
            b"\xff",
        ] {
            assert!(user_path(output).is_err());
        }
        assert_eq!(
            user_path(b"/var/folders/xx/user/0/\n").unwrap(),
            Path::new("/private/var/folders/xx/user/0")
        );
    }

    #[test]
    fn owner_checks_reject_mismatched_uid_and_entries_must_match_opened_inode() {
        let directory = fs::open("/", directory_flags(), Mode::empty()).unwrap();
        let mut stat = fs::fstat(&directory).unwrap();
        // 権限変更なしで所有者判定を検証 / Test ownership policy without chown.
        stat.st_uid = 123;
        stat.st_mode = 0o040700;
        assert!(validate_private_directory(&stat, 123).is_ok());
        assert_eq!(
            validate_private_directory(&stat, 124),
            Err(LockError::UnsafePath)
        );
        stat.st_mode = 0o100600;
        stat.st_nlink = 1;
        assert!(validate_lock_file(&stat, 123).is_ok());
        assert_eq!(validate_lock_file(&stat, 124), Err(LockError::UnsafePath));
        let child = fs::openat(&directory, "private", directory_flags(), Mode::empty()).unwrap();
        assert_eq!(
            same_entry(&directory, "private", &directory),
            Err(LockError::UnsafePath)
        );
        assert!(same_entry(&directory, "private", &child).is_ok());
    }
}
