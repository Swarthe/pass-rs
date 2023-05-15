use crate::{
    error::{Error, Result},
    find::{RecordPath, MatchKind}
};

use crate::util::{clip, proc};

use crate::util::{
    record::{Record, Node},
    proc::Process
};

use std::fmt::Display;

use std::time::Duration;

/// XXX: several paths
pub struct PrintTarget {
    paths: Vec<RecordPath>,
    mk: MatchKind
}

/// XXX: single paths
pub struct ClipTarget {
    path: RecordPath,
    mk: MatchKind,
    time: Duration
}

/// a result-like type that carries extra information on whether the process is
/// a child or a parent, if it was forked
///
/// This allows us to track what kind of process this is, even if the result is
/// an `Err`.
///
/// if .0 is None, then the process was not forked
/// if .0 is Some(p), then p determines whether or not process is forked
pub type ResultForked = (Option<Process>, Result<()>);

impl PrintTarget {
    pub fn new(paths: Vec<RecordPath>, mk: MatchKind) -> Self {
        Self { paths, mk }
    }

    pub fn print_values(self, data: &Node<Record>) {
        for p in self.paths {
            match p.find_item_or_default_in(data, self.mk) {
                Ok(item) => println!("{}", item.borrow().value()),
                Err(e) => Error::from(e).print_full()
            }
        }
    }

    pub fn print_lists(self, data: &Node<Record>) {
        print_each_spaced(self.paths, |p| {
            let rec = p.find_in(data, self.mk)?;

            Ok(Record::display_list(&rec))
        })
    }

    pub fn print_trees(self, data: &Node<Record>) {
        print_each_spaced(self.paths, |p| {
            let rec = p.find_in(data, self.mk)?;

            Ok(Record::display_tree(&rec))
        })
    }
}

impl ClipTarget {
    pub fn new(path: RecordPath, mk: MatchKind, time: Duration) -> Self {
        Self { path, mk, time }
    }

    /// Finds the target in `data` and copies it to the clipboard.
    ///
    /// Forks the process into a parent a child, the latter of which is
    /// responsible for preserving the clipboard. See [`clip_timed`] for more
    /// details.
    pub fn clip(self, data: &Node<Record>) -> ResultForked {
        let item_result = self.path
            .find_item_or_default_in(data, self.mk);

        let item = match item_result {
            Ok(i) => i,
            Err(e) => return (None, Err(e.into()))
        };

        let item = item.borrow();
        let value = item.value();

        clip_timed(value, self.time)
    }
}

/// Copies `text` to the primary clipboard, and clears it after `time`.
///
/// This operation is non-blocking for the calling process, as an identical
/// child process is started to preserve the clipboard as long as necessary
/// before continuing execution. The child process' memory is secured using
/// [`proc::secure_mem`].
///
/// Returns a value indicating whether the current process is the child or
/// parent. An expected usage pattern is to immediately end the child process
/// without it performing any IO.
pub fn clip_timed(text: &str, time: Duration) -> ResultForked {
    use clip::Clipboard;

    // SAFETY: Forking the process is completely safe because ours is
    // single-threaded.
    let proc_result = unsafe {
        // Since we will not modify memory allocated by the parent process, the
        // kernel should be able to apply COW optimisations, allowing for a low
        // performance penalty.
        proc::fork()
    };

    let proc = match proc_result {
        Ok(p) => p,
        Err(e) => return (None, Err(Error::StartingProcess(e)))
    };

    if proc == Process::Child {
        // TODO: use `try` blocks once available
        let result = (|| -> Result<()> {
            // The child process does not inherit the parent's memory
            // protections, so they must be reapplied.
            proc::secure_mem()
                .map_err(Error::SecuringMemory)?;

            Clipboard::new()?
                .hold(text, time)?;

            Ok(())
        })();

        (Some(proc), result.map_err(Error::from))
    } else {
        (Some(proc), Ok(()))
    }
}

/// Applies 'f' to each element of `paths` and prints the result separated with
/// empty lines.
///
/// If `f` returns an error, it is printed and execution continues
fn print_each_spaced<F, D>(paths: Vec<RecordPath>, f: F)
    where
        F: Fn(RecordPath) -> Result<D>,
        D: Display
{
    let mut paths = paths.into_iter();

    if let Some(p) = paths.next() {
        match f(p) {
            Ok(d) => println!("{d}"),
            Err(e) => e.print_full()
        }

        for p in paths {
            println!();

            match f(p) {
                Ok(d) => println!("{d}"),
                Err(e) => e.print_full()
            }
        }
    }
}
