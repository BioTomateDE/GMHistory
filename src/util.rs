use crate::error::*;
use std::io::ErrorKind;

fn ignore_kind(result: std::io::Result<()>, kind: ErrorKind) -> MyResult<()> {
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == kind => Ok(()),
        Err(e) => Err(Error::from(e)),
    }
}

pub fn ignore_notfound(result: std::io::Result<()>) -> MyResult<()> {
    ignore_kind(result, ErrorKind::NotFound)
}

pub fn ignore_exists(result: std::io::Result<()>) -> MyResult<()> {
    ignore_kind(result, ErrorKind::AlreadyExists)
}
