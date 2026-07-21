//! Language-neutral contracts for out-of-process audio codec plugins.

mod contract;
mod framing;

pub use contract::*;
pub use framing::*;

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;
