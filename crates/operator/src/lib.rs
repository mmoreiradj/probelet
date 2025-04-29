use snafu::Snafu;

#[derive(Snafu, Debug)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub mod operator;
pub use crate::operator::*;
