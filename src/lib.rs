//! Run commands and transfer files in Daytona sandboxes.
//!
//! This unofficial SDK uses validated inputs, borrowed handles, and explicit
//! recovery errors. Start with the [first command tutorial](guide::getting_started).
//! It includes dependencies, credentials, a complete program, and expected output.
//!
//! # Choose a task
//!
//! | Task | Start here |
//! | --- | --- |
//! | Learn the inputs | [Validated values](guide::inputs) |
//! | Configure a client | [Credentials, endpoints, and limits](guide::configuration) |
//! | Run a command | [First command](guide::getting_started), then [results](guide::commands) |
//! | Configure scoped work | [Source, resources, network](guide::configured_scope) |
//! | Run a longer command | [Command budgets](guide::long_commands) |
//! | Run agent jobs | [Bounded supervisor](guide::agent_jobs) |
//! | Transfer a file | [Upload and download](guide::files) |
//! | Handle failures | [Work and cleanup](guide::errors) |
//! | Manage a resource manually | [Lifecycle](guide::lifecycle) |
//! | Recover after interruption | [Reconciliation](guide::recovery) |
//! | Generate code with Pi | [Coding agent](guide::pi_task) |
//! | Diagnose a command | [Deliberate output inspection](guide::errors::diagnose_failure) |
//! | Run concurrent work | [Ownership](guide::ownership) |
//!
//! # A small workflow
//!
//! Given a configured [`Daytona`] client, this function returns owned output after
//! confirming sandbox deletion. The [complete tutorial](guide::getting_started)
//! shows client setup and explicit execution. Running it creates a billable resource.
#![doc = "\n```rust,no_run\n"]
#![doc = include_str!("../examples/scoped.rs")]
#![doc = "\n```\n"]
//!
//! # Requirements and scope
//!
//! Requires Rust 1.98.0 and a Tokio runtime. The Cargo package is `daytona-rs`;
//! use `daytona` as the dependency alias and Rust import name.
//!
//! Creation, observation, reconciliation, shell execution, file transfer, and
//! deletion are supported. See [supported scope](guide::troubleshooting) for limits.
//!
//! # Contracts to keep in mind
//!
//! - [`CheckedOutput`] requires an explicit zero exit code.
//! - A timeout limits waiting; it does not prove remote work stopped.
//! - Scope cleanup runs after normal completion, including work failure.
//! - Cancellation and process termination do not run asynchronous cleanup.
//! - Save a [`CreationReference`] before creation when restart recovery matters.
//! - Remote observations can become stale. Types do not freeze provider state.
//!
//! The [learning guide](guide) explains these contracts step by step.
#![forbid(unsafe_code)]
#![deny(unused_must_use, missing_docs)]
#![deny(rustdoc::broken_intra_doc_links, rustdoc::invalid_codeblock_attributes)]
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

pub mod guide;

#[doc = include_str!("../docs/compile-contracts.md")]
pub mod contracts {}
