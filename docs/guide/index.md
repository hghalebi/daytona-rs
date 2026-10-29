# Learn the Daytona SDK

Choose the path that fits your task. Each lesson includes setup, complete code,
an expected result, one exercise, and the next step.

- **Try it:** [run your first command](crate::guide::getting_started).
- **Run a coding agent:** [generate code with Pi, then delete the sandbox](crate::guide::pi_task).
- **Learn it:** [inputs](crate::guide::inputs) → [client setup](crate::guide::configuration) → [first command](crate::guide::getting_started) → [command results](crate::guide::commands) → [files](crate::guide::files).
- **Handle failures:** [work and cleanup](crate::guide::errors) → [inspect failure output](crate::guide::errors::diagnose_failure) → [preserve work](crate::guide::errors::preserve_work) → [recovery](crate::guide::recovery).
- **Build larger workflows:** [long commands](crate::guide::long_commands) → [configured scopes](crate::guide::configured_scope) → [bounded jobs](crate::guide::agent_jobs) → [Pi](crate::guide::pi_task).
- **Reference:** [ownership](crate::guide::ownership), [manual lifecycle](crate::guide::lifecycle), [configuration](crate::guide::configuration), and [troubleshooting](crate::guide::troubleshooting).

The examples use one recurring task: run a command in a temporary sandbox and
inspect its result. Local lessons run without Daytona access. Remote lessons
require explicit execution and a real key. Ordinary tests do not create resources.

## Terms used in this guide

| Term | Meaning |
| --- | --- |
| Client | Owns the HTTP transport and credentials |
| Handle | Owns an ID and borrows the client |
| Scope | Owns a temporary sandbox and performs normal-completion cleanup |
| Receipt | Records provider acknowledgement and a known identity |
| Observation | Records remote state at a point in time |
| Reference | Correlation data saved for recovery |
| Reconciliation | A read-only lookup to resolve an uncertain creation |
| Confirmed deletion | HTTP 404 or an observed destroyed state |

Read a method’s **Errors** and **Cancellation** sections before adding retry or
cancellation behavior. Values and types protect local contracts. Provider state,
permissions, and resource quotas still need runtime checks.
