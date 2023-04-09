mod ir;

use super::{
    secret::Erase,
    user_io::Style
};

use std::{fmt, mem};

use std::{
    fmt::Display,
    cmp::Ordering
};

use std::{
    collections::BTreeMap,
    rc::{Rc, Weak},
    cell::RefCell
};

pub use ir::Ir;

pub enum Record {
    Group(Node<Group>),
    Item(Node<Item>)
}

pub struct Group {
    /// XXX: static reference to avoid lifetimes (cant deserialise then)
    /// always valid as long as the child records exist (which is always the
    /// case if this group exists),
    /// but not truly static
    /// -- btreemap is always ordered
    members: BTreeMap<&'static str, Node<Record>>,
    meta: Metadata
}

pub struct Item {
    value: String,
    meta: Metadata
}

pub struct Metadata {
    /// XXX: should not modified be if a parent exists (to avoid invalidating
    /// the hashmap)
    name: String,
    parent: Option<WeakNode<Group>>,
}

#[derive(Debug)]
pub enum Error {
    Serialisation(ron::error::Error),
    Deserialisation(ron::error::SpannedError),
    NotFound,
    MultipleMatches,
    AlreadyExists,
}

pub type Node<T> = Rc<RefCell<T>>;
/// XXX: non owning, prevents cycle
type WeakNode<T> = Weak<RefCell<T>>;

pub type Result<T> = std::result::Result<T, Error>;

impl Record {
    pub fn from(ir: Ir) -> Node<Self> {
        match ir {
            Ir::Group { name, members, metadata: _ } => {
                let group = new_node(Group {
                    members: BTreeMap::new(),
                    meta: Metadata::for_root(name)
                });

                group.borrow_mut().members = members.into_iter().map(|ir| {
                    // TODO: technically unsound because the child (and its
                    // name) might be deallocated before the reference when
                    // removed or dropped

                    // SAFETY: We guarantee that the group will only keep `name`
                    // as long as `rec` is owned by it (and that the reference
                    // is therefore valid). The name will never be modified, so
                    // it will never be reallocated. Therefore, an immutable
                    // reference is valid. Furthermore, the record and its name
                    // will never be moved as it is kept behind an `Rc`.
                    let name = unsafe {
                        std::mem::transmute::<_, &'static str>(ir.name())
                    };

                    let rec = Record::with_parent(ir, &group);

                    (name, rec)
                }).collect::<BTreeMap<_, _>>();

                new_node(Record::Group(group))
            }

            Ir::Item { name, value, metadata: _ } => {
                new_node(Record::Item(new_node(Item {
                    value,
                    meta: Metadata::for_root(name)
                })))
            }
        }
    }

    pub fn new_group(name: String) -> Node<Self> {
        new_node(Record::Group(Group::new(name)))
    }

    pub fn new_item(name: String, value: String) -> Node<Self> {
        new_node(Record::Item(Item::new(name, value)))
    }

    pub fn display_list(this: &Node<Self>) -> impl Display {
        DisplayList(Rc::clone(this))
    }

    pub fn display_tree(this: &Node<Self>) -> impl Display {
        DisplayTree(Rc::clone(this))
    }

    pub fn do_with_meta<O, R>(&self, op: O) -> R
        where
            O: FnOnce(&Metadata) -> R
    {
        match self {
            Self::Group(g) => op(&g.borrow().meta),
            Self::Item(i) => op(&i.borrow().meta)
        }
    }

    pub fn parent(&self) -> Option<Node<Group>> {
        match self {
            Self::Group(g) => g.borrow().parent(),
            Self::Item(i) => i.borrow().parent()
        }
    }
}

// `Erase` is already implemented for `Node<T>` where `T` implements `Erase`.
// Like this, a `Secret` containing a `Node<Record>` can be created.
impl Erase for Record {
    #[inline(never)]
    fn erase(&mut self) {
        match self {
            Self::Group(g) => g.erase(),
            Self::Item(i) => i.erase()
        }
    }
}

impl Group {
    pub fn new(name: String) -> Node<Self> {
        new_node(Self {
            members: BTreeMap::new(),
            meta: Metadata::for_root(name)
        })
    }

    pub fn name(&self) -> &str {
        self.meta.name()
    }

    pub fn parent(&self) -> Option<Node<Group>> {
        self.meta.parent()
    }

    pub fn get(this: &Node<Self>, name: &str) -> Result<Node<Record>> {
        let this = this.borrow();

        let result = this.members
            .get(name)
            .ok_or(Error::NotFound)?;

        Ok(Rc::clone(result))
    }

    pub fn get_fuzzy(
        this: &Node<Self>,
        name_pat: &str
    ) -> Result<Node<Record>> {
        let this = this.borrow();
        let mut first_match = Option::<Match>::None;
        let mut members_iter = this.members.iter();

        for (name, rec) in &mut members_iter {
            if let Some(m) = Match::make(name_pat, name, rec) {
                first_match = Some(m);
                break;
            }
        }

        let mut best_match = first_match
            .ok_or(Error::NotFound)?;

        let mut have_multiple_matches = false;

        // Iterate through the remaining `Records`s.
        for (name, rec) in members_iter {
            if let Some(m) = Match::make(name_pat, name, rec) {
                match m.cmp_score(&best_match) {
                    Ordering::Greater => {
                        best_match = m;
                        // No other match exists yet at this new highest score.
                        have_multiple_matches = false;
                    }

                    Ordering::Equal => have_multiple_matches = true,
                    Ordering::Less => continue
                }
            }
        }

        if have_multiple_matches {
            Err(Error::MultipleMatches)
        } else {
            Ok(Rc::clone(best_match.val))
        }
    }

    /// XXX: fails if record of same name already exists, and rec is left
    /// unchanged
    pub fn insert(this: &Node<Self>, rec: &Node<Record>) -> Result<()> {
        let members = &mut this.borrow_mut().members;

        // TODO: use `BTreeMap::try_insert` once available
        let name = rec.borrow_mut().mutate_meta(|meta| {
            let name = meta.name.as_str();

            // If the insertion cannot be done, we return before modifying
            // 'rec'.
            if members.contains_key(name) {
                return Err(Error::AlreadyExists);
            }

            meta.parent = Some(Rc::downgrade(this));

            // SAFETY: Same as with `Record::from`.
            Ok(unsafe {
                mem::transmute::<_, &'static str>(name)
            })
        })?;

        members.insert(name, Rc::clone(rec));
        Ok(())
    }

    /// mutably borrows the removed record (may panic)
    pub fn remove(&mut self, name: &str) -> Result<Node<Record>> {
        let removed = self.members
            .remove(name)
            .ok_or(Error::NotFound)?;

        removed.borrow().set_parent(None);

        Ok(removed)
    }
}

impl Erase for Group {
    #[inline(never)]
    fn erase(&mut self) {
        self.members.erase();
        self.meta.erase();
    }
}

impl Item {
    pub fn new(name: String, value: String) -> Node<Self> {
        new_node(Self {
            value,
            meta: Metadata::for_root(name)
        })
    }

    pub fn name(&self) -> &str {
        self.meta.name()
    }

    pub fn value(&self) -> &String {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut String {
        &mut self.value
    }

    pub fn parent(&self) -> Option<Node<Group>> {
        self.meta.parent()
    }
}

impl Erase for Item {
    #[inline(never)]
    fn erase(&mut self) {
        self.value.erase();
        self.meta.erase();
    }
}

impl Metadata {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parent(&self) -> Option<Node<Group>> {
        let parent = &self.parent.as_ref()?;

        // A record's parent cannot have been dropped before the record itself.
        Some(parent.upgrade().unwrap())
    }
}

impl Erase for Metadata {
    #[inline(never)]
    fn erase(&mut self) {
        self.name.erase();
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            Deserialisation(e) =>
                write!(f, "{e}"),
            Serialisation(e) =>
                write!(f, "{e}"),
            NotFound =>
                write!(f, "record not found"),
            MultipleMatches =>
                write!(f, "multiple matches found"),
            AlreadyExists =>
                write!(f, "record already exists"),
        }
    }
}

impl Record {
    fn with_parent(ir: Ir, parent: &Node<Group>) -> Node<Self> {
        let result = Record::from(ir);

        result.borrow_mut().set_parent(Rc::downgrade(parent));
        result
    }

    fn mutate_meta<O, R>(&self, op: O) -> R
        where
            O: FnOnce(&mut Metadata) -> R
    {
        match self {
            Self::Group(g) => op(&mut g.borrow_mut().meta),
            Self::Item(i) => op(&mut i.borrow_mut().meta)
        }
    }

    fn set_parent<P>(&self, p: P)
        where
            P: Into<Option<WeakNode<Group>>>
    {
        self.mutate_meta(|meta| meta.parent = p.into());
    }
}

impl Metadata {
    fn for_root(name: String) -> Self {
        Self { name, parent: None }
    }
}

fn new_node<T>(v: T) -> Node<T> {
    Rc::new(RefCell::new(v))
}

/// XXX: doesnt display values
///   displays one layer, like unix `ls`
///   doesnt leak any actual data
struct DisplayList(Node<Record>);

/// XXX: doesnt display values
///   displays all layers, like unix `tree`
///   doesnt leak any actual data
struct DisplayTree(Node<Record>);

struct Match<'r> {
    val: &'r Node<Record>,
    score: isize
}

impl Display for DisplayList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0.borrow() {
            Record::Group(g) => {
                let g = g.borrow();
                let mut members_iter = g.members.iter();

                if let Some((name, rec)) = members_iter.next() {
                    rec.borrow().fmt_name(f, name)?;

                    for (name, rec) in members_iter {
                        writeln!(f)?;
                        rec.borrow().fmt_name(f, name)?;
                    }
                }
            }

            Record::Item(i) => write!(f, "{}", i.borrow().name())?
        }

        Ok(())
    }
}

impl Display for DisplayTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0.borrow() {
            Record::Group(g) => {
                let g = g.borrow();

                write!(f, "{}", g.name().as_title())?;
                g.fmt_as_branch(f, &mut String::new())
            }

            Record::Item(i) => write!(f, "{}", i.borrow().name())
        }
    }
}

impl<'r> Match<'r> {
    /// XXX: matches `pattern` to `target`
    /// - if made: returns `Self` containing `target`
    /// - otherwise: returns `None`
    fn make(pattern: &str, name: &str, val: &'r Node<Record>) -> Option<Self> {
        use sublime_fuzzy::best_match;

        let score = best_match(pattern, name)?.score();

        Some(Self { val, score })
    }

    fn cmp_score(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl Record {
    fn fmt_name(&self, f: &mut fmt::Formatter, name: &str) -> fmt::Result {
        match self {
            Record::Group(_) => write!(f, "{}", name.as_heading()),
            Record::Item(_) => write!(f, "{}", name)
        }
    }
}

impl Group {
    /// XXX: always prints leading newline, unless `self` is empty
    /// recursively formats the entire group
    /// `buffer` is reset to state before passed when func returns
    /// if called on root group, `buffer` should be empty
    fn fmt_as_branch(
        &self,
        dest: &mut fmt::Formatter,
        buffer: &mut String
    ) -> fmt::Result {
        const BAR: &str      = "\u{2502}   ";
        const SPACE: &str    = "    ";
        const FORK: &str     = "\u{251C}\u{2500}\u{2500} ";
        const FORK_END: &str = "\u{2514}\u{2500}\u{2500} ";

        // `peekable()` allows us to track if we are at the last member.
        let mut members_iter = self.members.iter().peekable();

        #[allow(clippy::write_with_newline)]
        while let Some((name, rec)) = members_iter.next() {
            write!(dest, "\n")?;
            write!(dest, "{buffer}")?;

            match members_iter.peek() {
                Some(_) => write!(dest, "{FORK}")?,
                None => write!(dest, "{FORK_END}")?
            };

            rec.borrow().fmt_name(dest, name)?;

            if let Record::Group(g) = &*rec.borrow() {
                let old_len = buffer.len();

                match members_iter.peek() {
                    Some(_) => buffer.push_str(BAR),
                    None    => buffer.push_str(SPACE),
                }

                g.borrow().fmt_as_branch(dest, buffer)?;
                buffer.truncate(old_len);       // Revert `buf`.
            }
        }

        Ok(())
    }
}
