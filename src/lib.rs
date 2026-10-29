//! A concrete client, validated inputs, borrowed handles, and explicit recovery.
//!
//! See the [contract examples](contracts) for permitted workflows and compiler
//! rejections. Remote observations are snapshots, never permanent capabilities.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
mod client;
mod create;
mod domain;
mod error;
mod execution;
mod scope;
mod transport;

pub use client::*;
pub use create::*;
pub use domain::*;
pub use error::*;
pub use execution::*;
pub use scope::*;

#[doc = include_str!("../docs/compile-contracts.md")]
pub mod contracts {}
