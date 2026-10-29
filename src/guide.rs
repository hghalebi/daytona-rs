//! Task-based lessons and complete examples.
#![doc = include_str!("../docs/guide/index.md")]

#[doc = include_str!("../docs/guide/inputs.md")]
#[doc = "\n```rust\n"]
#[doc = include_str!("../examples/validate_inputs.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/inputs-notes.md")]
pub mod inputs {}

#[doc = include_str!("../docs/guide/configuration.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/configure_client.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/configuration-notes.md")]
pub mod configuration {}

#[doc = include_str!("../docs/guide/getting_started.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/first_command.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/getting_started-notes.md")]
pub mod getting_started {}

#[doc = include_str!("../docs/guide/commands.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/inspect_exit.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/commands-notes.md")]
pub mod commands {}

#[doc = include_str!("../docs/guide/pi_task.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/pi_task.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/pi_task-notes.md")]
pub mod pi_task {}

#[doc = include_str!("../docs/guide/lifecycle.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/manual_lifecycle.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/lifecycle-notes.md")]
pub mod lifecycle {}

#[doc = include_str!("../docs/guide/recovery.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/recover_creation.rs")]
#[doc = "\n```\n"]
#[doc = include_str!("../docs/guide/recovery-notes.md")]
pub mod recovery {}

#[doc = include_str!("../docs/guide/files.md")]
pub mod files {
    #[doc = include_str!("../docs/guide/upload.md")]
    #[doc = "\n```rust,no_run\n"]
    #[doc = include_str!("../examples/upload_file.rs")]
    #[doc = "\n```\n"]
    #[doc = include_str!("../docs/guide/upload-notes.md")]
    pub mod upload {}

    #[doc = include_str!("../docs/guide/download.md")]
    #[doc = "\n```rust,no_run\n"]
    #[doc = include_str!("../examples/download_file.rs")]
    #[doc = "\n```\n"]
    #[doc = include_str!("../docs/guide/download-notes.md")]
    pub mod download {}
}

#[doc = include_str!("../docs/guide/errors.md")]
pub mod errors {
    #[doc = include_str!("../docs/guide/diagnose_failure.md")]
    #[doc = "\n```rust,no_run\n"]
    #[doc = include_str!("../examples/diagnose_failure.rs")]
    #[doc = "\n```\n"]
    pub mod diagnose_failure {}

    #[doc = include_str!("../docs/guide/preserve_work.md")]
    #[doc = "\n```rust,no_run\n"]
    #[doc = include_str!("../examples/preserve_work.rs")]
    #[doc = "\n```\n"]
    #[doc = include_str!("../docs/guide/preserve_work-notes.md")]
    pub mod preserve_work {}
}

#[doc = include_str!("../docs/guide/ownership.md")]
pub mod ownership {
    #[doc = include_str!("../docs/guide/concurrent_commands.md")]
    #[doc = "\n```rust,no_run\n"]
    #[doc = include_str!("../examples/concurrent_commands.rs")]
    #[doc = "\n```\n"]
    #[doc = include_str!("../docs/guide/concurrent_commands-notes.md")]
    pub mod concurrent_commands {}
}

#[doc = include_str!("../docs/guide/troubleshooting.md")]
pub mod troubleshooting {}

#[doc = include_str!("../docs/guide/configured_scope.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/configured_scope.rs")]
#[doc = "\n```\n"]
pub mod configured_scope {}

#[doc = include_str!("../docs/guide/long_commands.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/long_command.rs")]
#[doc = "\n```\n"]
pub mod long_commands {}

#[doc = include_str!("../docs/guide/agent_jobs.md")]
#[doc = "\n```rust,no_run\n"]
#[doc = include_str!("../examples/agent_jobs.rs")]
#[doc = "\n```\n"]
pub mod agent_jobs {}

#[doc = include_str!("../docs/guide/macros.md")]
pub mod macros {}
