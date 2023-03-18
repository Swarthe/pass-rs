//! XXX: intermediate representation

use super::{Record, Error, Node};

use crate::util::secret::Erase;
use crate::util::secret::Secret;

use serde::{Serialize, Deserialize};

use std::fmt;

use std::fmt::Display;

use std::{
    collections::BTreeMap,
    rc::Rc
};

/// XXX: intermediate representation
#[derive(Serialize, Deserialize)]
pub enum Ir {
    Group {
        name: String,
        members: Vec<Ir>,
        #[allow(unused)]    // May be useful later.
        metadata: Metadata
    },
    Item {
        name: String,
        value: String,
        #[allow(unused)]
        metadata: Metadata
    }
}

type Metadata = BTreeMap<String, String>;

type Result<T> = std::result::Result<T, Error>;

impl Ir {
    pub fn name(&self) -> &str {
        match self {
            Self::Group { name, .. } => name,
            Self::Item { name, .. } => name
        }
    }

    pub fn clone_from(rec: &Node<Record>) -> Self {
        match &*rec.borrow() {
            Record::Group(g) => {
                let g = g.borrow();

                let members = g.members.values()
                    .map(Self::clone_from)
                    .collect();

                Self::Group {
                    name: g.meta.name.clone(),
                    members,
                    metadata: BTreeMap::new()
                }
            }

            Record::Item(i) => {
                let i = i.borrow();

                Self::Item {
                    name: i.meta.name.clone(),
                    value: i.value.clone(),
                    metadata: BTreeMap::new()
                }
            }
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        ron::from_str(s)
            .map_err(Error::Deserialisation)
    }

    pub fn to_string(&self) -> Result<String> {
        ron::to_string(self)
            .map_err(Error::Serialisation)
    }
}

impl From<Node<Record>> for Ir {
    fn from(r: Node<Record>) -> Self {
        Ir::from(take(r))
    }
}

impl From<Record> for Ir {
    fn from(r: Record) -> Self {
        match r {
            Record::Group(g) => {
                let g = take(g);

                let members = g.members.into_values()
                    .map(Ir::from)
                    .collect();

                Self::Group {
                    name: g.meta.name,
                    members,
                    metadata: BTreeMap::new()
                }
            }

            Record::Item(i) => {
                let i = take(i);

                Self::Item {
                    name: i.meta.name,
                    value: i.value,
                    metadata: BTreeMap::new()
                }
            }
        }
    }
}

impl Display for Ir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ron::ser::PrettyConfig;

        let conf = PrettyConfig::default();

        // TODO: find a way to write directly to formatter using
        // `to_writer_pretty` or something. this approach creates allocates a
        // string and requires erasing it
        let serial = Secret::new(
            ron::ser::to_string_pretty(self, conf)
                .map_err(Error::Serialisation)
                .unwrap()
        );

        write!(f, "{}", *serial)
    }
}

impl Erase for Ir {
    #[inline(never)]
    fn erase(&mut self) {
        match self {
            Self::Group { name, members, metadata: _ } => {
                name.erase();
                members.erase();
                //metadata.erase();
            }

            Self::Item { name, value, metadata: _ } => {
                name.erase();
                value.erase();
                //metadata.erase();
            }
        }
    }
}

/// XXX: may panic
fn take<T>(v: Node<T>) -> T {
    Rc::try_unwrap(v)
        .unwrap_or_else(|_| panic!("Rc is owned by someone else"))
        .into_inner()
}
