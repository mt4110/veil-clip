#![cfg(target_os = "macos")]

#[path = "../src/instance_lock.rs"]
mod instance_lock;

use std::fs::{self, DirBuilder, File};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use instance_lock::{InstanceLock, LockError};

fn fixture() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("lock-test-{}-{nonce}", std::process::id()));
    DirBuilder::new().mode(0o700).create(&path).unwrap();
    // 検証後も削除せずtarget内に保持する / Retain fixtures; do not delete.
    path
}

// 同じ試験実行ファイルを独立した子プロセスとして起動する。
// Run the test executable as an independent process; no clipboard module exists here.
#[test]
fn lock_worker() {
    if std::env::var_os("VEIL_CLIP_LOCK_WORKER").is_none() {
        return;
    }
    let result = match std::env::var_os("VEIL_CLIP_LOCK_FIXTURE") {
        Some(path) => InstanceLock::acquire_in(&PathBuf::from(path)),
        None => InstanceLock::acquire(),
    };
    let _lock = match result {
        Ok(lock) => lock,
        Err(_) => std::process::exit(3),
    };
    println!("LOCK_READY");
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}

struct Worker(Child);
impl Worker {
    fn start(path: &std::path::Path) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lock_worker", "--nocapture"])
            .env("VEIL_CLIP_LOCK_WORKER", "1")
            .env("VEIL_CLIP_LOCK_FIXTURE", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if line.unwrap() == "LOCK_READY" {
                    let _ = sender.send(());
                }
            }
        });
        let mut worker = Self(child);
        if receiver.recv_timeout(Duration::from_secs(5)).is_err() {
            let _ = worker.0.kill();
            panic!("child did not acquire fixture lock");
        }
        worker
    }
    fn finish(mut self, forced: bool) {
        if forced {
            self.0.kill().unwrap();
        } else {
            self.0.stdin.take().unwrap().write_all(b"\n").unwrap();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert_eq!(status.success(), !forced);
                break;
            }
            assert!(std::time::Instant::now() < deadline, "child exit timeout");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[test]
fn independent_process_contention_and_normal_exit_release() {
    let path = fixture();
    let worker = Worker::start(&path);
    assert!(matches!(
        InstanceLock::acquire_in(&path),
        Err(LockError::Occupied)
    ));
    let lock_path = path.join("veil-clip/apply.lock");
    let before = fs::metadata(&lock_path).unwrap();
    assert_eq!(before.mode() & 0o7777, 0o600);
    assert_eq!(
        fs::metadata(path.join("veil-clip")).unwrap().mode() & 0o7777,
        0o700
    );
    worker.finish(false);
    let _lock = InstanceLock::acquire_in(&path).unwrap();
    let after = fs::metadata(lock_path).unwrap();
    assert_eq!(before.ino(), after.ino());
    assert_eq!(after.len(), 0);
}

#[test]
fn forced_exit_releases_without_unlinking_or_recreating_file() {
    let path = fixture();
    let worker = Worker::start(&path);
    let lock_path = path.join("veil-clip/apply.lock");
    let inode = fs::metadata(&lock_path).unwrap().ino();
    worker.finish(true);
    let _lock = InstanceLock::acquire_in(&path).unwrap();
    assert_eq!(fs::metadata(lock_path).unwrap().ino(), inode);
}

#[test]
fn existing_content_is_never_truncated_and_handle_drop_releases() {
    let path = fixture();
    drop(InstanceLock::acquire_in(&path).unwrap());
    let lock_path = path.join("veil-clip/apply.lock");
    let mut file = fs::OpenOptions::new().write(true).open(&lock_path).unwrap();
    file.write_all(b"synthetic preexisting bytes").unwrap();
    drop(file);
    drop(InstanceLock::acquire_in(&path).unwrap());
    assert_eq!(fs::read(lock_path).unwrap(), b"synthetic preexisting bytes");
}

#[test]
fn rejects_directory_permissions_and_directory_links() {
    let path = fixture();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        InstanceLock::acquire_in(&path),
        Err(LockError::UnsafePath)
    ));
    let path = fixture();
    DirBuilder::new()
        .mode(0o755)
        .create(path.join("veil-clip"))
        .unwrap();
    assert!(matches!(
        InstanceLock::acquire_in(&path),
        Err(LockError::UnsafePath)
    ));
    let path = fixture();
    let destination = fixture();
    std::os::unix::fs::symlink(&destination, path.join("veil-clip")).unwrap();
    assert!(InstanceLock::acquire_in(&path).is_err());
    assert!(!destination.join("apply.lock").exists());
    let path = fixture();
    std::os::unix::fs::symlink(&destination, path.join("linked-base")).unwrap();
    assert!(InstanceLock::acquire_in(&path.join("linked-base")).is_err());
}

#[test]
fn rejects_file_links_permissions_and_nonregular_entries() {
    let path = fixture();
    drop(InstanceLock::acquire_in(&path).unwrap());
    fs::set_permissions(
        path.join("veil-clip/apply.lock"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(matches!(
        InstanceLock::acquire_in(&path),
        Err(LockError::UnsafePath)
    ));

    let path = fixture();
    DirBuilder::new()
        .mode(0o700)
        .create(path.join("veil-clip"))
        .unwrap();
    let destination = path.join("untouched");
    File::create(&destination)
        .unwrap()
        .write_all(b"synthetic bytes")
        .unwrap();
    std::os::unix::fs::symlink(&destination, path.join("veil-clip/apply.lock")).unwrap();
    assert!(InstanceLock::acquire_in(&path).is_err());
    assert_eq!(fs::read(destination).unwrap(), b"synthetic bytes");

    let path = fixture();
    drop(InstanceLock::acquire_in(&path).unwrap());
    fs::hard_link(path.join("veil-clip/apply.lock"), path.join("alias")).unwrap();
    assert!(matches!(
        InstanceLock::acquire_in(&path),
        Err(LockError::UnsafePath)
    ));

    let path = fixture();
    DirBuilder::new()
        .mode(0o700)
        .create(path.join("veil-clip"))
        .unwrap();
    fs::create_dir(path.join("veil-clip/apply.lock")).unwrap();
    assert!(InstanceLock::acquire_in(&path).is_err());
}
