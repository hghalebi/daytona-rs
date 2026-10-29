## Expected outcomes

Observe mode prints `Found sandbox: <id>` or `Unresolved; do not replay creation`.
Explicit delete also prints `Deletion confirmed` after absence is observed.
Invalid schemas and mismatched organization context fail before network access.

Reconciliation performs at most three read-only attempts within 15 seconds. Empty,
ambiguous, and paginated results remain unresolved. An empty result does not prove
that creation never happened. No path automatically resubmits creation.

The record stores the requested TTL and the local time intent was saved. Neither
is proof of provider creation time or actual expiry. Its cloud environment tag
cannot redirect credentials. Keep records private and use trusted current client
configuration when extending the lesson for self-hosted endpoints.

Exercise: change the saved organization, then run observe mode. It should fail
locally. Restore the original record before recovery. Next: [ownership and
concurrency](crate::guide::ownership).
