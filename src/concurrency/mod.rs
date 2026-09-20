//! Concurrency primitives more than one path needs.
//!
//! Nothing here knows about packages, references or upstream: a primitive that two
//! paths share has to be the same mechanism for both, or it is two mechanisms with one
//! name.

pub mod single_flight;

pub use single_flight::{Resolution, SingleFlight, Slot, Waiter};
