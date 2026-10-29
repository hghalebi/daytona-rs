# Upload one file

Copy a small local file into a temporary sandbox.

## Before you start

Read [the first command](crate::guide::getting_started). Supply an API key.
Run this local setup in the checkout. Noclobber protects an existing file:

```sh
(set -C; printf 'hello\n' > lesson-input.txt)
```

The upload overwrites the explicit remote path inside a new sandbox. Cleanup
deletes that sandbox, so the remote file is temporary. The local input stays.

## Run from the source checkout

```sh
cargo run --example upload_file -- --run-live
```

The [complete source](https://docs.rs/crate/daytona-rs/0.1.2/source/examples/upload_file.rs)
is shipped with the crate and included below.

## Complete example
