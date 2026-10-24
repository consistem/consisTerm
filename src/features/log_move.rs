//! Taking the transcripts along when the logs folder changes.
//!
//! Only the app's own files are moved: a transcript, a rotated one, an
//! export, every one of them named `<name>-YYYYMMDD-HHMMSS` and a `.log`,
//! `.txt` or `.html`. The folder can be pointed anywhere, Documents included, and a move
//! that swept up whatever else lives there would be the worst thing this
//! screen could do.
//!
//! The work runs on a thread of its own and reports file by file, so a few
//! hundred megabytes of transcript going to a network drive leaves the window
//! drawing - and the sessions being drained - the whole time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

/// The extensions the app writes into the logs folder.
const OURS: [&str; 3] = ["log", "txt", "html"];

/// Whether `name` is a file the app wrote: a stem ending in the timestamp
/// every transcript and export is given, and one of their extensions.
pub fn is_ours(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    if !OURS.contains(&ext.to_ascii_lowercase().as_str()) {
        return false;
    }
    // `-YYYYMMDD-HHMMSS` at the end.
    let bytes = stem.as_bytes();
    if bytes.len() < 17 {
        return false;
    }
    let tail = &bytes[bytes.len() - 15..];
    bytes[bytes.len() - 16] == b'-'
        && tail[8] == b'-'
        && tail[..8].iter().all(u8::is_ascii_digit)
        && tail[9..].iter().all(u8::is_ascii_digit)
}

/// What there is to move, worked out before asking.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub from: PathBuf,
    pub to: PathBuf,
    /// Each file, and its size in bytes.
    pub files: Vec<(PathBuf, u64)>,
}

impl Plan {
    /// The app's files in `from`, sorted by name so the order shown is the
    /// order a listing of the folder has. A folder that is not there has
    /// nothing in it.
    pub fn new(from: &Path, to: &Path) -> Plan {
        let mut files: Vec<(PathBuf, u64)> = std::fs::read_dir(from)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
            .filter(|entry| entry.file_name().to_str().is_some_and(is_ours))
            .map(|entry| {
                let size = entry.metadata().map_or(0, |m| m.len());
                (entry.path(), size)
            })
            .collect();
        files.sort();
        Plan {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            files,
        }
    }

    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|(_, size)| size).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// How far a move has got.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// The file being moved, by name.
    pub current: String,
    /// Files finished, moved or not.
    pub done: usize,
    pub bytes_done: u64,
    /// Files that could not be moved, and why. They stay where they were.
    pub failed: Vec<(String, String)>,
    /// The thread has stopped: every file tried, or the move called off.
    pub finished: bool,
    pub cancelled: bool,
}

/// A move under way: the latest word from its thread, and the way to call it
/// off.
pub struct Job {
    pub plan: Plan,
    pub progress: Progress,
    rx: Receiver<Progress>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    /// Starts moving `plan`'s files; `wake` is called whenever there is news,
    /// so a window that draws nothing of its own accord hears about it.
    pub fn start(plan: Plan, wake: impl Fn() + Send + 'static) -> Job {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            plan: plan.clone(),
            progress: Progress::default(),
            rx,
            cancel: cancel.clone(),
        };
        std::thread::spawn(move || run(&plan, &tx, &cancel, &wake));
        job
    }

    /// Takes in whatever the thread has said since the last frame.
    pub fn poll(&mut self) {
        while let Ok(progress) = self.rx.try_recv() {
            self.progress = progress;
        }
    }

    /// Stops after the file being moved now: a file is never left half
    /// copied.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn run(plan: &Plan, tx: &Sender<Progress>, cancel: &AtomicBool, wake: &dyn Fn()) {
    let mut progress = Progress::default();
    let send = |p: &Progress| {
        let _ = tx.send(p.clone());
        wake();
    };
    if let Err(e) = std::fs::create_dir_all(&plan.to) {
        for (path, _) in &plan.files {
            progress.failed.push((file_name(path), e.to_string()));
        }
        progress.finished = true;
        send(&progress);
        return;
    }
    for (path, size) in &plan.files {
        if cancel.load(Ordering::Relaxed) {
            progress.cancelled = true;
            break;
        }
        progress.current = file_name(path);
        send(&progress);
        let target = free_name(&plan.to, &progress.current);
        if let Err(e) = move_file(path, &target, |copied| {
            let mut during = progress.clone();
            during.bytes_done += copied;
            send(&during);
        }) {
            progress
                .failed
                .push((progress.current.clone(), e.to_string()));
        }
        progress.done += 1;
        progress.bytes_done += size;
    }
    progress.current.clear();
    progress.finished = true;
    send(&progress);
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `name` in `dir`, or `name (2)`, `name (3)` and so on when a file of that
/// name is there already - a file moved must never land on another.
fn free_name(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}).{ext}")))
        .find(|candidate| !candidate.exists())
        .expect("some number is free")
}

/// Moves one file: a rename where the two folders share a drive, and
/// otherwise a copy in pieces - reported as it goes - and only then the
/// original removed, so a copy that fails part way loses nothing.
fn move_file(from: &Path, to: &Path, mut copied: impl FnMut(u64)) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    use std::io::{Read, Write};
    let mut source = std::fs::File::open(from)?;
    let result = (|| {
        let mut target = std::fs::File::create(to)?;
        let mut buffer = vec![0u8; 1 << 20];
        let mut total = 0;
        loop {
            let read = source.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            target.write_all(&buffer[..read])?;
            total += read as u64;
            copied(total);
        }
        target.sync_all()
    })();
    drop(source);
    match result {
        Ok(()) => std::fs::remove_file(from),
        Err(e) => {
            let _ = std::fs::remove_file(to);
            Err(e)
        }
    }
}

/// Bytes as a person reads them: `840 B`, `12.4 KB`, `3.1 MB`.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nit-logmove-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_the_apps_own_files_count_as_ours() {
        assert!(is_ours("CONSISTEM-20261023-194412.log"));
        assert!(is_ours("RDB76-TR-20261023-194412-20261023-201500.log"));
        assert!(is_ours("COMP80-20261023-194412.txt"));
        assert!(is_ours("COMP80-20261023-194412.HTML"));
        assert!(!is_ours("notes.txt"));
        assert!(!is_ours("report-2026.html"));
        assert!(!is_ours("COMP80-20261023-194412.docx"));
        assert!(!is_ours("20261023-194412.log"));
        assert!(!is_ours("x-2026102a-194412.log"));
    }

    /// Everything the app wrote goes, and nothing else does - the folder may
    /// be one the user keeps other things in.
    #[test]
    fn a_move_takes_the_apps_files_and_leaves_the_rest() {
        let from = tempdir("from");
        let to = tempdir("to").join("new");
        std::fs::write(from.join("A-20261023-101010.log"), b"one").unwrap();
        std::fs::write(from.join("B-20261023-101011.txt"), b"two!").unwrap();
        std::fs::write(from.join("mine.txt"), b"keep me").unwrap();
        let plan = Plan::new(&from, &to);
        assert_eq!(plan.files.len(), 2);
        assert_eq!(plan.bytes(), 7);
        let mut job = Job::start(plan, || ());
        while !job.progress.finished {
            job.poll();
            std::thread::yield_now();
        }
        assert!(job.progress.failed.is_empty(), "{:?}", job.progress.failed);
        assert_eq!(job.progress.done, 2);
        assert_eq!(job.progress.bytes_done, 7);
        assert_eq!(
            std::fs::read(to.join("A-20261023-101010.log")).unwrap(),
            b"one"
        );
        assert!(!from.join("A-20261023-101010.log").exists());
        assert!(from.join("mine.txt").exists(), "not ours, not moved");
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(to.parent().unwrap());
    }

    /// A file of the same name already in the new folder is not written over.
    #[test]
    fn a_file_already_there_is_kept_and_the_moved_one_renamed() {
        let from = tempdir("clash-from");
        let to = tempdir("clash-to");
        std::fs::write(from.join("A-20261023-101010.log"), b"moved").unwrap();
        std::fs::write(to.join("A-20261023-101010.log"), b"there").unwrap();
        let mut job = Job::start(Plan::new(&from, &to), || ());
        while !job.progress.finished {
            job.poll();
            std::thread::yield_now();
        }
        assert_eq!(
            std::fs::read(to.join("A-20261023-101010.log")).unwrap(),
            b"there"
        );
        assert_eq!(
            std::fs::read(to.join("A-20261023-101010 (2).log")).unwrap(),
            b"moved"
        );
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_dir_all(&to);
    }

    #[test]
    fn sizes_read_as_people_say_them() {
        assert_eq!(size(840), "840 B");
        assert_eq!(size(12_700), "12.4 KB");
        assert_eq!(size(3_250_000), "3.1 MB");
    }
}
