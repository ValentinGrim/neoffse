//! Asking which executables have a live process.
//!
//! [`running`] takes executable names and returns those with a live process.
//! The module reads the process table and writes nothing. The two rules
//! behind it (the process name on Windows, the first command-line argument
//! under Wine or Proton on Linux) are in `docs/architecture.md#save-safety`.
//!
//! The scan is best effort. A process the system hides from the caller, as
//! in a sandbox with its own process table, stays invisible. An empty result
//! is a warning that nothing was seen, never proof that no game runs.
//!
//! The `Debug` output of [`ProcessInfo`] holds the first command-line
//! argument of a process, which can hold a home path. A caller must not log
//! it; nothing in the crate does.

use std::ffi::OsStr;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Which field of a process names its executable. A later version may add
/// variants, so a `match` needs a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Key {
    /// The whole [`ProcessInfo::name`] is the executable name (Windows).
    Name,
    /// The last component of [`ProcessInfo::argv0`], split on `/` and `\`, is
    /// the executable name (Linux, where the game runs under Wine or Proton).
    CommandLine,
}

impl Key {
    /// `Name` on Windows, `CommandLine` elsewhere. The only place the
    /// process rules branch on the operating system.
    #[must_use]
    pub fn platform() -> Self {
        if cfg!(windows) { Self::Name } else { Self::CommandLine }
    }
}

/// What the scan keeps of one process. Built by [`snapshot`], or by hand in a test.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessInfo {
    /// The process name as the system reports it (cut to 15 bytes on Linux).
    pub name: String,
    /// The first non-empty command-line argument. `None` when the scan did
    /// not read it ([`Key::Name`]), or the system gave none. `sysinfo` drops
    /// empty parts and trims ASCII whitespace when it splits the command line.
    pub argv0: Option<String>,
    /// A thread or a kernel thread. Never a match.
    pub is_thread: bool,
}

/// Reads the live process table through `sysinfo`: one [`ProcessInfo`] per
/// process, in no particular order. A [`Key::CommandLine`] snapshot reads each
/// command line; a [`Key::Name`] snapshot reads none. On Windows `sysinfo`
/// still opens each process handle to read its start time, but under
/// [`Key::Name`] the scan reads no command line, environment or working
/// folder from another process, and the name comes from the process
/// snapshot of the system. Lossy UTF-8 for every text.
#[must_use]
pub fn snapshot(key: Key) -> Vec<ProcessInfo> {
    // Tasks off: on Linux each thread would otherwise be listed as a process
    // under the thread's name. Kernel threads (`kthreadd`, `kworker/...`) are
    // ordinary entries of the process table and still carry `thread_kind`.
    let mut kind = ProcessRefreshKind::nothing().without_tasks();
    // The command line stays unread under `Name`: on Windows reading it can
    // need administrator rights and opens the memory of foreign processes.
    if key == Key::CommandLine {
        kind = kind.with_cmd(UpdateKind::OnlyIfNotSet);
    }
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    system
        .processes()
        .values()
        .map(|process| ProcessInfo {
            name: lossy(process.name()),
            argv0: if key == Key::CommandLine {
                process.cmd().first().map(|argument| lossy(argument))
            } else {
                None
            },
            is_thread: process.thread_kind().is_some(),
        })
        .collect()
}

fn lossy(text: &OsStr) -> String {
    text.to_string_lossy().into_owned()
}

/// The pure rule. Returns the entries of `executables` that have a matching
/// process, in the order and with the multiplicity of `executables`.
/// A thread never matches; an empty executable name matches nothing; the
/// comparison is ASCII case-insensitive and the result keeps the caller's spelling.
#[must_use]
pub fn matching<'a>(processes: &[ProcessInfo], key: Key, executables: &[&'a str]) -> Vec<&'a str> {
    executables
        .iter()
        .copied()
        .filter(|executable| {
            !executable.is_empty()
                && processes
                    .iter()
                    .filter(|process| !process.is_thread)
                    .any(|process| identity(process, key).is_some_and(|name| name.eq_ignore_ascii_case(executable)))
        })
        .collect()
}

/// The text compared with an executable name. Each key reads its own field
/// only. Under `CommandLine` the 15-byte `name` is ignored (it cuts every
/// long title to the same prefix) and so is `exe()` (it names the Wine
/// loader). Whole-name equality on the last path component, not a suffix
/// test: `NOT FINAL FANTASY.exe` must not match `FINAL FANTASY.exe`.
/// Wine rewrites `argv[0]` to a Windows path, so both separators split.
/// See `docs/architecture.md#save-safety`.
fn identity(process: &ProcessInfo, key: Key) -> Option<&str> {
    match key {
        Key::Name => Some(process.name.as_str()),
        Key::CommandLine => process.argv0.as_deref().and_then(|argv0| argv0.rsplit(['/', '\\']).next()),
    }
}

/// [`snapshot`] and [`matching`] with [`Key::platform`]. Returns early with an
/// empty list when `executables` is empty.
#[must_use]
pub fn running<'a>(executables: &[&'a str]) -> Vec<&'a str> {
    if executables.is_empty() {
        return Vec::new();
    }
    let key = Key::platform();
    matching(&snapshot(key), key, executables)
}
