//this module is for utillity of the cli
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
