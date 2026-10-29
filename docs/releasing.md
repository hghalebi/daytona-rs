# Releasing daytona-rs

The first prepared version is **0.1.0**. The package is `daytona-rs`, the library
import is `daytona`, and the internal `source-policy` package has `publish = false`.
Only the crates.io registry is allowed by the SDK manifest.

## Package contents and documentation

The manifest's include list ships source, tests, examples, documentation,
Cargo manifests/lockfile, the pinned toolchain, Clippy configuration, changelog,
and both license texts. `README.md` and `docs/compile-contracts.md` are compile-time
inputs through `include_str!`; both must remain in the archive.

Cargo normalizes workspace-inherited lints into the packaged manifest. The
unpublished policy tool is a checkout-only development dependency, not an SDK
dependency. Run policy checks from the checkout; test the SDK itself again from
the extracted archive to detect workspace coupling.

The public upstream is [hghalebi/daytona-rs](https://github.com/hghalebi/daytona-rs).
The manifest's repository and homepage fields point there. The documentation URL
is the package's docs.rs destination; its availability depends on a successful
registry release and docs.rs build.

## Local preparation

Run from the repository root using Rust 1.98.0:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test -p daytona-rs --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc -p daytona-rs --no-deps --locked
cargo install --path tools/source-policy --root target/policy --locked
target/policy/bin/source-policy .
cargo package -p daytona-rs --list --locked --allow-dirty
cargo package -p daytona-rs --locked --allow-dirty
cargo publish -p daytona-rs --registry crates-io --dry-run --locked --allow-dirty
```

`--allow-dirty` is for reviewing this local, uncommitted preparation. It does not
skip Cargo's package build verification or upload anything during a dry run. For
the actual release, commit the reviewed files and use a clean tree without this
flag. Do not use `--no-verify` to bypass a failing package build.

`cargo package` creates `target/package/daytona-rs-0.1.0.crate` and verifies an
extracted copy. Cargo may stage the publish dry run archive in a temporary
subdirectory instead; the explicit package command gives a stable artifact path. Run the shipped tests and documentation against that copy:

```sh
cargo test --manifest-path target/package/daytona-rs-0.1.0/Cargo.toml --all-targets --locked
cargo test --manifest-path target/package/daytona-rs-0.1.0/Cargo.toml --doc --locked
```

For stronger isolation, extract the `.crate` archive to a temporary directory
outside this workspace and run the same checks there. Inspect the archive file
list before publishing; no credential material, `.plan`, `target`, or `.git`
content belongs in it. Future releases must update the version, changelog,
archive paths in these commands.

## Publication

Publication is a separate explicit action. Before uploading, confirm the release
version and package ownership, verify the public repository metadata, update the changelog's prepared status, and commit the reviewed release.
A read-only crates.io lookup returning 404 means the name is currently unlisted;
it neither reserves the name nor guarantees registry acceptance.

Use the publisher's verified crates.io account and Cargo's credential provider or
interactive `cargo login`. Do not paste a registry token into documentation, shell
arguments, source files, or CI logs. A dry run does not establish publisher account
permissions or validate a future docs.rs build.

From the clean release checkout, the explicit upload command is:

```sh
cargo publish -p daytona-rs --registry crates-io --locked
```

After upload, verify the registry version and checksum, install the dependency in
a separate consumer project, and check that docs.rs built the API documentation.
Tag the exact published commit as `v0.1.0` once the uploaded version is confirmed.
The CI workflow only performs verification and a dry run; it has no upload token
and does not publish on pushes or tags.

Reference: [Cargo's publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html).
