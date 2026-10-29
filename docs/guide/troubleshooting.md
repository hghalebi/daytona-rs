# Find the next action

Start with the typed error. Avoid printing secrets or retrying a mutation merely
because the HTTP response was lost.

| Symptom | Check | Next action |
| --- | --- | --- |
| Missing environment variable | Application setup | Export `DAYTONA_API_KEY`; dotenv is not loaded implicitly |
| Authentication rejected | Key validity | Replace/reacquire the key without logging it |
| Authorization rejected | Organization and operation permissions | Use the intended account and granted permissions |
| `Busy` | In-flight request limit | Reduce concurrent work; this request was not sent |
| Request timeout | HTTP and operation deadlines | Inspect the operation’s retained recovery data |
| Unknown creation | Saved reference | Reconcile; do not automatically POST again |
| Missing exit status | Provider response | Preserve unknown status; do not report success |
| Nonzero exit | Command code and captured text | Handle the program’s failure |
| Response/file limit | Configured capture bound | Use a suitable explicit limit or smaller result |
| Download destination exists | Local path | Inspect or move it; download will not overwrite it |
| Partial write | Local storage and permissions | Inspect the incomplete file before retrying |
| Cleanup deadline | Sandbox ID and acceptance | Inspect remote state; deletion is not confirmed |
| Unknown remote state | Provider evolution | Preserve the unknown value; do not infer readiness |

## Supported scope

This release supports creation, readiness observation, reconciliation, shell
execution, bounded file transfer, and confirmed deletion. Session management,
preview discovery, snapshot management, volumes, and computer-use operations are
not implemented. Some reference/credential types exist for those domains without
corresponding remote operations.

## Ask for help

Open an issue with the SDK version, operation, sanitized error variant, and a
minimal reproducible example. Exclude keys, shell output containing secrets,
private recovery records, and environment files. Use the repository’s private
vulnerability-reporting channel for security issues.
