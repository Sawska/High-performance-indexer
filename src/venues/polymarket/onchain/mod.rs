//! Polygon log indexing: the on-chain counterpart to the REST scraper.
//!
//! The REST APIs report what Polymarket's backend believes; these logs are what
//! the chain actually settled. Both are written so the two can be compared.

pub mod abi;
pub mod consts;
pub mod events;
pub mod rpc;

#[cfg(test)]
mod live_tests;
