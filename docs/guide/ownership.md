# Understand ownership and concurrency

The client owns its connection pool. A sandbox handle owns its identity and borrows
the client. The client must stay alive while the handle is used.

A scope owns the sandbox. It lends a reference to your async callback. Your returned
output must own what it needs after the callback ends. This lets the scope delete
the resource before returning the result.

## Run borrowed futures together

See [two concurrent commands](crate::guide::ownership::concurrent_commands). Both futures borrow one handle.
They complete within its lifetime. The API does not require cloning for this use.
A scoped callback is not promised to produce a Send future for every capture.

For long-lived application state, store IDs and workflow data. Borrow a client and
construct handles at operation boundaries. Avoid a struct that owns a client and
stores handles borrowing that same field.

## Cancellation is a separate effect

Dropping a future stops local waiting. It does not undo remote effects, stop a
command, or perform asynchronous deletion. Before a cancellable creation, persist
its reference. Before dropping an owned handle during another operation, retain
its identity. See [recovery](crate::guide::recovery).

## Why the types help

Newtypes keep different IDs and units distinct. Typestate restricts local steps,
such as submission before validation. Results make failures explicit. Ownership
makes transfers of responsibility visible. An observation cannot guarantee future
remote state. The [compile contracts](crate::contracts) show accepted and rejected
uses. The deeper implementation rationale is in the repository’s contract guide.
