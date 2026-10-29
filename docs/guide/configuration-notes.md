## Expected result and explanation

```text
Client configured; no API request sent
```

The environment variable enters through `ApiKey::new`. The client consumes the
credential and configuration. It does not test the credential with Daytona yet.
`drop(client)` ends local ownership; it does not delete a remote sandbox.

Try changing the request deadline to 15 seconds. Local construction still works.
The deadline affects future HTTP operations, not client initialization.

If `DAYTONA_API_KEY` is missing, the program returns an environment error. If the
key contains spaces, validation rejects it. Never print a real key to diagnose it.
Next: [run your first command](crate::guide::getting_started).

## Defaults and units

| Setting | Default | What it bounds |
| --- | --- | --- |
| Concurrency | 16 | In-flight HTTP operations; fail-fast admission by default |
| Response capture | 4 MiB | JSON, including command output |
| File transfer | 16 MiB | File bytes buffered in memory |
| HTTP request | 60 seconds | Request and response body |
| Readiness wait | 60 seconds in a scope | Time to observe ready |
| Cleanup wait | 30 seconds in a scope | Delete submission and confirmation |
| Poll interval | 250 milliseconds | Delay between observations |
| Execution | 10 seconds sent by default | Provider command time limit |
| Sandbox TTL | Required | Provider lifetime, in minutes |

[`ClientConfig`](crate::ClientConfig) changes HTTP limits. Scope methods change
readiness and cleanup deadlines. [`Shell::execution_timeout`](crate::Shell::execution_timeout)
sets the provider limit and an explicit request timeout. Without that setting,
the command request uses the client HTTP deadline. The outer client cap applies
even with an explicit execution timeout. The earlier deadline wins. Raising only
the client cap does not add grace to the command request timeout. No timeout proves a process
stopped. A TTL is a fallback, not proof of deletion.

For separate provider and HTTP limits, use [CommandBudget](crate::CommandBudget)
as shown in [long commands](crate::guide::long_commands). The last budget or
execution_timeout setter wins.

For self-hosted or regional deployments, construct both trusted base URLs with
[`Endpoint`](crate::Endpoint) and pass them to `ClientConfig::new`. HTTPS is required
except for loopback HTTP. Credentials are sent to the explicitly configured
API and toolbox endpoints. Redirects are disabled.

Use [`Resources`](crate::Resources) for CPU, memory, and disk with an image source.
Default and named snapshots retain their own resources; validation rejects custom
allocations with those sources. Memory and disk use
provider GB fields. [`NetworkPolicy`](crate::NetworkPolicy) chooses provider
behavior, blocking, or allow lists. Provider permissions and quotas remain runtime
checks. See the constructors for exact accepted ranges.

## Lifecycle capacity

ClientConfig::lifecycle_capacity accepts LifecycleCapacity::Wait with a validated
PendingLifecycleLimit and RequestTimeout. It queues only observations and deletion,
within the same total HTTP cap. Ordinary execution, creation, and transfer requests
still fail fast. A full pending queue returns Busy; expired waiting returns
CapacityDeadline. Neither transmits that request. Readiness and cleanup deadlines
include capacity waiting. Cancelling a waiter releases its queue slot.

The default remains FailFast. Bounded waiting helps short bursts; it cannot promise
cleanup while another operation occupies capacity beyond the cleanup deadline.
DeleteAcceptance::NotSubmitted describes only this attempt. A prior attempt may
already have taken effect. An acknowledged DELETE remains acknowledged if a later
confirmation poll fails.
