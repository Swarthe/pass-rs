use MatchKind::{Fuzzy, Exact};

use crate::config::DEFAULT_ITEM;

use crate::util::record;

use crate::util::record::{
    Record, Group, Item,
    Node
};

use std::fmt;

use std::fmt::Display;

use std::rc::Rc;

/// XXX: an empty path is root
#[derive(Debug)]
pub struct RecordPath(String);

/// The method by which a record is searched for.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// Matches each element of the path to record names using a fuzzy matching
    /// algorithm.
    #[default]
    Fuzzy,
    /// Matches each element of the path to record names.
    Exact
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    // Clippy recommends we wrap `record::Error` in a `Box` because of its size.
    /// Could not find record matching `pat` in group of name `in_group`.
    NotFound { e: Box<record::Error>, pat: String, in_group: String },
    /// Expected a group, but got `rec` matching `pat` instead.
    ///
    /// `pat` is None if the root group was matched, or if an exact match was
    /// found (in which case the pattern was the name).
    NotAGroup { name: String, pat: Option<String> },
    /// Expected an item, but got `rec` matching `pat` instead.
    NotAnItem { name: String, pat: Option<String> }
}

pub type Result<T> = std::result::Result<T, Error>;

pub type SplitResult<T> = std::result::Result<(T, T), T>;

impl RecordPath {
    pub const DELIM: char = '.';

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0
            .split(Self::DELIM)
            .filter(|e| !e.is_empty())      // Ignore extraneous delimiters.
    }

    /// if self is not root, returns its parent path, and the trailing path
    /// element. returned trailing element won't have any delimiters
    /// if self is root, returns err and does not modify
    /// if self only has 1 element, leading will be empty and trailing will
    /// contain it
    pub fn split_last(mut self) -> SplitResult<RecordPath> {
        let trailing = match self.iter().last() {
            Some(e) => RecordPath::from(e.to_owned()),
            None => return Err(self)
        };

        self.strip_last();

        Ok((self, trailing))
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn find_in(
        &self,
        rec: &Node<Record>,
        mk: MatchKind
    ) -> Result<Node<Record>> {
        let found = self.find_rec_in(rec, mk)?;

        Ok(found.rec)
    }

    pub fn find_group_in(
        &self,
        rec: &Node<Record>,
        mk: MatchKind
    ) -> Result<Node<Group>> {
        let FoundRecord { rec, matched_pat } = self.find_rec_in(rec, mk)?;
        let rec = rec.borrow();

        match &*rec {
            Record::Group(g) => Ok(Rc::clone(g)),

            Record::Item(i) =>  Err(Error::NotAGroup {
                name: i.borrow().name().to_owned(),
                pat: matched_pat
            })
        }
    }

    pub fn find_item_in(
        &self,
        rec: &Node<Record>,
        mk: MatchKind
    ) -> Result<Node<Item>> {
        let FoundRecord { rec, matched_pat } = self.find_rec_in(rec, mk)?;
        let rec = rec.borrow();

        match &*rec {
            Record::Group(g) => Err(Error::NotAnItem {
                name: g.borrow().name().to_owned(),
                pat: matched_pat
            }),

            Record::Item(i) => Ok(Rc::clone(i))
        }
    }

    /// XXX:
    /// if item is found, return that
    /// if group is found, return `DEFAULT_ITEM` directly inside it if it exists
    pub fn find_item_or_default_in(
        &self,
        rec: &Node<Record>,
        mk: MatchKind
    ) -> Result<Node<Item>> {
        let found = self.find_in(rec, mk)?;
        let found_ref = &*found.borrow();

        match found_ref {
            Record::Group(_) => RecordPath::from(DEFAULT_ITEM)
                .find_item_in(&found, Exact),

            Record::Item(i) => Ok(Rc::clone(i))
        }
    }
}

impl<S: Into<String>> From<S> for RecordPath {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

impl Display for RecordPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl MatchKind {
    pub fn from_str(s: &str) -> Option<Self> {
        // "exact" and "fuzzy" are completely distinct strings, so the following
        // won't have unexpected results.
        if "exact".starts_with(s) {
            Some(Exact)
        } else if "fuzzy".starts_with(s) {
            Some(Fuzzy)
        } else {
            None
        }
    }
}

impl Display for MatchKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Fuzzy => f.write_str("fuzzy"),
            Exact => f.write_str("exact")
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            NotFound { e, pat, in_group } =>
                write!(f, "'{pat}' in group '{in_group}': {e}"),

            // No need to specify the pattern if it is equal to the record name.
            NotAGroup { name, pat } => match pat {
                Some(pat) => write!(f, "'{pat}': '{name}' is not a group"),
                None      => write!(f, "'{name}' is not a group")
            }

            // No need to specify the pattern if it is equal to the record name.
            NotAnItem { name, pat } => match pat {
                Some(pat) => write!(f, "'{pat}': '{name}' is not an item"),
                None      => write!(f, "'{name}' is not an item")
            }
        }
    }
}

struct FoundRecord {
    rec: Node<Record>,
    /// None if `rec` is root.
    matched_pat: Option<String>
}

impl RecordPath {
    /// Finds a record matching the target within `rec` or its children.
    fn find_rec_in(
        &self,
        rec: &Node<Record>,
        mk: MatchKind
    ) -> Result<FoundRecord> {
        let mut rec = Rc::clone(rec);
        let mut matched_pat = Option::<&str>::None;

        for pat in self.iter().peekable() {
            let found = match &*rec.borrow() {
                Record::Group(g) => match mk {
                    Fuzzy => Group::get_fuzzy(g, pat),
                    Exact => Group::get(g, pat)
                }.map_err(|e| Error::NotFound {
                    e: Box::new(e),
                    pat: pat.to_owned(),
                    in_group: g.borrow().name().to_owned()
                })?,

                Record::Item(i) => return Err(Error::NotAGroup {
                    name: i.borrow().name().to_owned(),
                    pat: match mk {
                        Fuzzy => Some(pat.to_owned()),
                        Exact => None
                    }
                })
            };

            rec = found;
            matched_pat = Some(pat);
        }

        let matched_pat = matched_pat
            .filter(|_| mk != Exact)    // The pattern equals the record name.
            .map(ToOwned::to_owned);

        Ok(FoundRecord { rec, matched_pat })
    }
}

impl RecordPath {
    ///  if self is not root, removes its last path element
    ///  (effectively turning it into its parent path)
    ///  if it is root, removes any extraneous delimiters if they exist
    fn strip_last(&mut self) {
        // TODO: make this code cleaner if possible

        // Remove the extraneous delimiters if they exist to access the
        // trailing path element.
        if let Some(Self::DELIM) = self.last_char() {
            while let Some(Self::DELIM) = self.last_char() {
                self.0.pop();
            }
        }

        // TODO: replace with `while self.last_char().is_some_and(|c| c != Self::DELIM)`
        //  or let chains when available

        // Remove the trailing path element.
        while let Some(c) = self.last_char() {
            if c == Self::DELIM { break; }
            self.0.pop();
        }
    }

    fn last_char(&self) -> Option<char> {
        self.0.chars().last()
    }
}
