use crate::find;

use crate::util::record;

use crate::util::{
    record::{Record, Node, Ir},
    secret::Secret
};

use std::{
    fmt,
    str
};

use std::fmt::Display;

pub enum Error {
    NonUtf8Data(str::Utf8Error),
    Deserialisation(record::Error),
    Serialisation(record::Error),
    InvalidRecord(find::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<record::Error> for Error {
    fn from(e: record::Error) -> Self {
        Self::Deserialisation(e)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Self::NonUtf8Data(e)
    }
}

pub fn parse(bytes: &[u8]) -> Result<Node<Record>> {
    let serial = str::from_utf8(bytes)?;
    // This needs not be wrapped in a `Secret` because it will infallibly be
    // converted by value into a `Record`.
    let ir = Ir::from_str(serial)?;

    Ok(Record::from(ir))
}

pub fn str_from(bytes: &[u8]) -> Result<&str> {
    Ok(str::from_utf8(bytes)?)
}

pub fn ir_from(bytes: &[u8]) -> Result<Ir> {
    let serial = str_from(bytes)?;

    Ir::from_str(serial)
        .map_err(Error::Deserialisation)
}

pub fn bytes_from(rec_secret: Secret<Node<Record>>) -> Result<Vec<u8>> {
    let rec = rec_secret.into_inner();
    let ir = Secret::new(Ir::from(rec));

    let result = ir.to_string()
        .map_err(Error::Serialisation)?
        .into_bytes();

    Ok(result)
}

/// XXX: returns Ok(()) if valid serial data
pub fn validate(s: &str) -> Result<()> {
    use find::Error::NotAGroup;

    let ir = Ir::from_str(s)?;
    let rec = Secret::new(Record::from(ir));
    let rec_ref = &*rec.borrow();

    match rec_ref {
        // The root group can obviously not be an item.
        Record::Item(i) => Err(Error::InvalidRecord(NotAGroup {
            name: i.borrow().name().to_owned(),
            pat: None
        })),

        Record::Group(_) => Ok(())
    }
}

/// XXX: empty group record in serial form
pub fn new_empty(name: String) -> String {
    let rec = Record::new_group(name);
    let ir = Secret::new(Ir::from(rec));

    // Serialising an empty `Record` should never fail.
    ir.to_string().unwrap()
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            NonUtf8Data(e)     => write!(f, "{e}"),
            Deserialisation(e) => write!(f, "{e}"),
            Serialisation(e)   => write!(f, "{e}"),
            InvalidRecord(e)   => write!(f, "{e}")
        }
    }
}
