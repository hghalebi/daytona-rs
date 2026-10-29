# Download one file

Save a known remote file to a new local path.

## Before you start

Read [upload](crate::guide::files::upload) and supply an API key. Ensure `lesson-output.txt`
does not already exist. The example creates its own remote fixture, so it does
not depend on the sandbox from the previous lesson.

## Run from the source checkout

```sh
cargo run --example download_file -- --run-live
```

The [complete source](https://docs.rs/crate/daytona-rs/0.2.0/source/examples/download_file.rs)
is shipped with the crate and included below.

## Complete example
