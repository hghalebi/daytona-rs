## Expected result and explanation

```text
Downloaded 6 bytes; sandbox deleted
```

The new local `lesson-output.txt` contains `hello` followed by a newline. It
remains after sandbox deletion. `download` never overwrites an existing destination.
The file is captured within the configured limit before local writing begins.

Try running the example again without moving the local file. Download returns an
I/O error instead of overwriting it; sandbox cleanup is still attempted. Inspect
or move the old file before choosing to run again.

A `PartialWrite` error or cancellation during writing can leave an incomplete local
file. Inspect it before retrying. Next: [work and cleanup errors](crate::guide::errors).
