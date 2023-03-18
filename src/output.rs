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

impl PrintTarget {
    pub fn new(paths: Vec<RecordPath>, mk: MatchKind) -> Self {
        Self { paths, mk }
    }

    pub fn print_names(self, data: &Node<Record>) {
        for p in self.paths {
            match p.find_item_or_default_in(data, self.mk) {
                Ok(item) => println!("{}", item.borrow().value()),
                // If a record is not found, inform the user and continue.
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

    pub fn clip(self, data: &Node<Record>) -> Result<Process> {
        let item = self.path.find_item_or_default_in(data, self.mk)?;
        let item = item.borrow();

        let value = item.value();

        clip_timed(value, self.time)
    }
}

/// Copies `text` to the primary clipboard, and clears it after `time`.
///
/// This operation is non-blocking for the calling process, as an identical
/// child process is started to preserve the clipboard as long as necessary
/// before continuing execution.
///
/// Returns a value indicating whether the current process is the child or
/// parent. An expected usage pattern is to immediately end the child process.
pub fn clip_timed(text: &str, time: Duration) -> Result<Process> {
    use crate::with_secured_mem;
    use clip::Clipboard;

    // SAFETY: Forking the process is completely safe because ours is
    // single-threaded.
    let proc = unsafe {
        // Since we will not modify memory allocated by the parent process, the
        // kernel should be able to apply COW optimisations, allowing for a low
        // performance penalty.
        proc::fork()
    }.map_err(Error::StartingProcess)?;

    if proc == Process::Child {
        // The child process does not inherit the parent's memory protections,
        // so they must be reapplied.
        with_secured_mem(|| {
            let mut clip = Clipboard::new()?;
            clip.hold(text, time)?;

            Ok(())
        })?;
    }

    Ok(proc)
}

/// XXX: applies `f` to each of `paths`, and prints results spaced out with
/// empty line
/// if error is encountered, prints and continues
fn print_each_spaced<F, D>(paths: Vec<RecordPath>, f: F)
    where
        F: Fn(RecordPath) -> Result<D>,
        D: Display
{
    let mut paths = paths.into_iter();

    if let Some(p) = paths.next() {
        // If an error was encountered, inform the user and continue.
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
