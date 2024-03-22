use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ElasticError {
    NoAlias,
    NoIndexFoundForAlias,
    MoreThanOneIndexFoundForAlias,
    IndexAlreadyExists,
}

impl Error for ElasticError {}

impl Display for ElasticError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Elastic error..")
    }
}
