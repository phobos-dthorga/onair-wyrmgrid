//! Language-neutral control and encoded-packet contracts for supervised audio providers.

mod contract;
mod framing;

pub use contract::*;
pub use framing::*;

#[cfg(test)]
#[path = "tests/unit.rs"]
mod tests;
