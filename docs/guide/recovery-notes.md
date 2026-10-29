## Expected result and explanation

The output is one of:

```text
Found sandbox: <sandbox-id>
```

```text
Unresolved; do not replay creation
```

`CreationReference` deserialization checks its ID. It does not manufacture an
acknowledgement. `reconcile` performs a read-only correlation lookup. A single
matching result yields a receipt; no match, multiple matches, or pagination remain
unresolved. Eventual consistency means an empty result cannot prove non-creation.

A found receipt can be inspected, awaited, or converted into a handle for explicit
cleanup. The example only prints its ID; the operator must decide the next action.
Do not delete an unfamiliar resource just because a lookup returned it.

Try reconciling a reference after confirmed deletion. An unresolved result is
possible; it never authorizes automatic replay. An invalid JSON file fails before
network access. No deliberately uncertain live create is needed to learn this flow.
Next: [ownership and concurrency](crate::guide::ownership).
