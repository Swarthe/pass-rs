mod cmd;

use Status::{Running, Stopped, Aborted, Clipped};

use cmd::{Cmd, ReadCmd, EditCmd, MetaCmd, OptVal};

use crate::{input, err, info, warn};

use crate::{error, output, serial};

use crate::find::MatchKind;

use crate::util::user_io;

use crate::util::secret::Erase;

use crate::util::{
    record::{Record, Group, Node, Ir},
    proc::Process,
    secret::Secret
};

use std::{io, mem, fmt};

use std::fmt::Display;

use std::time::Duration;

// TODO: perhaps add option for hiding input
pub struct Tui {
    conf: Config,
    /// Useful to avoid unnecessarily writing to a file if the data is
    /// unchanged.
    changes_made: bool,
    status: Status
}

pub struct Config {
    pub match_kind: MatchKind,
    pub clip_time: Duration
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Running the TUI.
    Running,
    /// Exited the TUI normally.
    Stopped,
    /// Exited the TUI abnormally.
    ///
    /// Signals that the pass file should not be written to.
    Aborted,
    /// Used in the clipboard holder process, to signal that the pass file
    /// should not be written to (thus avoiding a race condition).
    Clipped
}

pub type Error = error::Error;

pub type Result = error::Result<()>;

/// XXX: not hygienic
macro_rules! err_continue {
    ($($arg:tt)*) => {{
        err!($($arg)*);
        continue;
    }};
}

impl Tui {
    pub fn new(conf: Config) -> Self {
        Self {
            conf,
            changes_made: false,
            status: Stopped
        }
    }

    /// after normal return, status should not be `Running`
    pub fn run(&mut self, data: &Node<Record>) -> Result {
        use io::ErrorKind::UnexpectedEof;

        self.status = Running;

        while self.status == Running {
            match input!() {
                Ok(l) => {
                    let cmd = match Cmd::from_str(&l) {
                        Ok(Some(cmd)) => cmd,
                        Ok(None) => continue,
                        Err(e) => err_continue!("{e}")
                    };

                    if let Err(e) = cmd.exec(data, self) {
                        e.print_full();
                    }
                }

                Err(e) => if e.kind() == UnexpectedEof {
                    // Exit if the user closed the stream (C-d).
                    self.status = Stopped;
                } else {
                    self.status = Aborted;
                    return Err(Error::ReadingInput(e))
                }
            }
        }

        Ok(())
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn should_save_data(&self) -> bool {
        self.status == Stopped && self.changes_made
    }
}

impl Config {
    fn set(&mut self, opt: OptVal) {
        use OptVal::*;

        match opt {
            ClipTime(t) => self.clip_time = t,
            MatchKind(k) => self.match_kind = k
        }
    }
}

impl Display for Config {
    #[allow(clippy::write_with_newline)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Function alias.
        let name = <str as user_io::Style>::as_name;

        // Writes each element aligned and coloured.
        write!(f, "{} {}\n", name("match-kind :"), self.match_kind)?;
        write!(f, "{} {}", name("clip-time  :"), self.clip_time.as_secs())
    }
}

impl Cmd {
    fn exec(self, data: &Node<Record>, tui: &mut Tui) -> Result {
        use Cmd::*;

        match self {
            Read(cmd) => cmd.exec(data, tui),
            Edit(cmd) => cmd.exec(data, tui),
            Meta(cmd) => cmd.exec(tui),
        }
    }
}

impl ReadCmd {
    fn exec(self, data: &Node<Record>, tui: &mut Tui) -> Result {
        use ReadCmd::*;
        use output::{PrintTarget, ClipTarget};

        let conf = &tui.conf;

        match self {
            Show(paths) => PrintTarget::new(paths, conf.match_kind)
                .print_names(data),

            Clip(path) => {
                let proc = ClipTarget::new(path, conf.match_kind, conf.clip_time)
                    .clip(data)?;

                // The clipboard should exit immediately without performing IO.
                if proc == Process::Child {
                    tui.status = Clipped;
                }
            }

            List(paths) => match paths {
                Some(paths) => PrintTarget::new(paths, conf.match_kind)
                    .print_lists(data),
                None => println!("{}", Record::display_list(data))
            }

            Tree(paths) => match paths {
                Some(paths) => PrintTarget::new(paths, conf.match_kind)
                    .print_trees(data),
                None => println!("{}", Record::display_tree(data))
            }

            Export => {
                let ir = Secret::new(Ir::clone_from(data));
                println!("{}", *ir);
            }
        }

        Ok(())
    }
}

impl EditCmd {
    fn exec(self, data: &Node<Record>, tui: &mut Tui) -> Result {
        use EditCmd::*;

        // Exact matching is always used for editing, to prevent mistakes.
        // Furthermore, the default item and other kinds of path inference are
        // not used.
        const MK: MatchKind = MatchKind::Exact;

        if tui.conf.match_kind != MK {
            warn!("exact matching is always used for editing");
        }

        match self {
            Remove { paths } => for p in paths {
                let rec = match p.find_in(data, MK) {
                    Ok(r) => r,
                    Err(e) => err_continue!("{e}")
                };

                let parent = match rec.borrow().parent() {
                    Some(p) => p,
                    None => err_continue!("cannot remove root group '{p}'")
                };

                remove(rec, parent);
            }

            Move { src, dest } => {
                // TODO
                err!("unimplemented: '{src}', '{dest}'");
                // we will split path_2
            }

            Copy { src, dest } => {
                // TODO
                err!("unimplemented: '{src}', '{dest}'");
            }

            CreateItem { dest, name } => {
                let parent = dest.find_group_in(data, MK)?;
                let item = Record::new_item(name, input!("Value: ")?);

                insert(item, &parent)?;
            }

            CreateGroup { dest, name } => {
                let parent = dest.find_group_in(data, MK)?;
                let group = Record::new_group(name);

                insert(group, &parent)?;
            }

            ChangeValue { path } => {
                let item = path.find_item_in(data, MK)?;
                // We don't need to wrap this in a `Secret` because it will be
                // immediately and infallibly swapped into a protected record.
                let mut value = input!("New value: ")?;

                mem::swap(
                    item.borrow_mut().value_mut(),
                    &mut value
                );

                value.erase();
            }
        }

        tui.changes_made = true;

        Ok(())
    }
}

impl MetaCmd {
    fn exec(self, tui: &mut Tui) -> Result {
        use MetaCmd::*;

        match self {
            SetOpt(opt) => tui.conf.set(opt),
            ShowConfig => println!("{}", tui.conf),

            // TODO
            ShowUsage(verb) => {
                err!("unimplemented");

                println!("\
sh | show => Show,
cl | clip => Clip,
ls | list => List,
tr | tree => Tree,
ex | export => Export,

rm | remove => Remove,
mv | move => Move,
cp | copy => Copy,
mg | mkgrp => CreateGroup,
mi | mkitm => CreateItem,
cv | chval => ChangeValue,

so | setopt => SetOption,
sc | showconf => ShowConfig,
hp | help => ShowUsage,
et | exit => Exit,
at | abort => Abort"
                );
            }

            Exit => tui.status = Stopped,
            Abort => tui.status = Aborted,
        }

        Ok(())
    }
}

/// Removes `rec` from `parent` and erases it.
///
/// Panics if `rec` is not a member of `parent`
fn remove(rec: Node<Record>, parent: Node<Group>) {
    let name = clone_name(&rec);

    // The record can only be removed if its parent group is its only owner.
    drop(rec);

    // `rec` is known to be a child of `parent`, so it can be infallibly
    // removed.
    let mut removed = parent.borrow_mut().remove(&name).unwrap();
    removed.erase();

}

/// erase `rec` on failure
fn insert(mut rec: Node<Record>, group: &Node<Group>) -> Result {
    match Group::insert(group, &rec) {
        Ok(()) => Ok(()),

        Err(e) => {
            let name = clone_name(&rec);
            rec.erase();

            Err(Error::AddingRecord(e, name))
        }
    }
}

// TODO: find a way to do this without cloning (hard)
fn clone_name(rec: &Node<Record>) -> String {
    match &*rec.borrow() {
        Record::Group(g) => g.borrow().name().to_owned(),
        Record::Item(i) => i.borrow().name().to_owned(),
    }
}
