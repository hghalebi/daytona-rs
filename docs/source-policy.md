# Source policy

The workspace forbids unsafe code and denies unused must-use results. Clippy runs
with warnings denied across the library, policy executable, tests, and examples.
Its pinned configuration disables test/constant exceptions for unwrap, expect,
panic, and indexing, and disallows explicit Clone calls and runtime assertions.

The compiled `source-policy` executable parses owned Rust files with Syn and
inspects their token trees, and applies the same checks to Rust Markdown fences. Negative fixtures are strings in the policy test;
they are intentionally rejected inputs, not compiled SDK implementations.

The checker rejects:

- The entire `unwrap*`/`expect*` spelling family and assertion macros.
- `Clone`/`Copy` derives or implementations, including fully qualified calls,
  `Arc::clone`, `clone_from`, `cloned`, `to_owned`, `to_vec`, and repetition helpers
  (including the cloning form of `vec![value; count]`).
- `panic`, `todo`, `unimplemented`, `unreachable`, unsafe code, and `.ok()`.
- Wildcard `let _` discards, indexing/slicing expressions, and arithmetic operators.
- Local lint suppression and unreviewed function-like or derive macros.
- Manual standard Error implementations, and error definitions without an Error
  derive. Imported `Error` derives refer to thiserror in the reviewed sources.

Owned macro definitions are rejected. The closed function-like macro list contains
formatting, collection literals, matching, included documentation, environment
constants, and Tokio borrowing/concurrency macros. The derive list contains only
Debug/equality, Serde, and thiserror derives. Adding a macro requires reviewing its
expansion and updating the policy. Generated protocol code must pass the same
checks; there is no generated-source exemption. Currently the protocol DTOs are
small handwritten adapters and there is no build-time code generator.

`.err()` is allowed when success data is intentionally unneeded and the error is
retained, as in setup cleanup reporting. `.ok()` is forbidden because it discards
the failure. Fallible operations are handled explicitly, propagated with `?`, or
stored as independent outcomes; logging alone is not a recovery policy.

This is an enforced source contract, not a proof of mathematical totality. It
cannot infer arbitrary semantic duplication, know every dependency's behavior,
prove remote cleanup, or guarantee that user callbacks terminate. Source review
must still detect semantic evasion such as allocating a new String solely to
reconstruct an owned identity. Buffer copies and wire serialization at adapters
are necessary boundary operations, not duplication of exclusive domain authority.

External procedural macro implementations, runtimes, allocators, and other
third-party code remain outside the owned-source proof boundary. Clippy adds
semantic checks on compiled code; compiler misuse tests and protocol tests verify
separate architectural and behavioral obligations. No test result is described
as a universal panic-freedom or exactly-once guarantee.
