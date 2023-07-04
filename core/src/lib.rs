pub mod indexers;
// mod find;
mod iterator;
mod parser;
mod result;
mod span;
mod tests;


// pub use find::{FindResult, find};
pub use iterator::{CharIndexer, CharIterator, Marker};
pub use parser::{Error, ParseResult, parse};
pub use result::*;
