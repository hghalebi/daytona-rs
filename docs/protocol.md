# Protocol boundary

Reviewed 2026-10-29 against the provider's published schemas:

| Source | SHA-256 of downloaded bytes |
| --- | --- |
| [Main API](https://www.daytona.io/docs/openapi.json) | `b366008fe15187432cc88cd86753ecd9c1aed59f544dca1ae402609534aea6d9` |
| [Toolbox API](https://www.daytona.io/docs/toolbox-openapi.json) | `f0ea3913734c88d94ee97512a119ac2b646a6b0b98d8f637156c2da784c061c4` |

Schemas can evolve independently of this SDK. Unknown object properties are
ignored; unknown remote state values are retained. Required identities, shell
result bodies, and pagination metadata must be present and valid. An absent exit
status is an explicit incomplete execution outcome.

| Operation | Route relative to configured base | Wire contract |
| --- | --- | --- |
| Create | `POST sandbox` | `ttlMinutes`, private preview, `autoDeleteInterval`, one source, optional resources/network policy, operation correlation label |
| Observe | `GET sandbox/{id}` | Verify returned ID; preserve missing/unknown state; 404 means absent |
| Reconcile | `GET sandbox?labels=...&limit=2` | Verify matching label, one result, and null next cursor; never replay |
| Delete | `DELETE sandbox/{id}`, then GET | Accepted status does not establish confirmed disappearance |
| Shell | `POST {id}/process/execute` | `command`, seconds `timeout`, optional `cwd`; `result` and optional `exitCode` |
| Download | `GET {id}/files/download?path=...` | Bounded bytes; local create-new |
| Upload | `POST {id}/files/upload-v2?path=...` | Raw `application/octet-stream` body; remote overwrite |

The current schema explicitly defines TTL as wall-clock minutes since creation,
regardless of sandbox state. Auto-delete is a separate stopped-state policy; the
default request uses zero for immediate deletion after stopping. Image creation
uses Dockerfile text in `buildInfo`; this SDK does not assemble external build
contexts or push image layers.

Cloud API base: `https://app.daytona.io/api/`. Cloud toolbox base:
`https://proxy.app.daytona.io/toolbox/`. The caller explicitly configures other
trusted endpoints. Provider response `toolboxProxyUrl` values are not followed
with credentials. Authentication uses Bearer; JWT also sends
`X-Daytona-Organization-ID`.

Mutation responses 400, 401, 403, 404, 405, 409, 413, 415, 422 and 429 classify as
provider rejection. HTTP 408, redirects, server failures, malformed successful
acknowledgements, disconnects, and deadline expiry are conservatively uncertain.
Only failures before executing the HTTP request (construction/admission) return
an unsubmitted request. No HTTP retry or redirect policy is enabled.

Raw remote response bodies and HTTP library diagnostics do not enter public
errors. Error variants preserve failure classification and recovery values;
status failures retain their HTTP status. This deliberately trades provider free
text for reliable diagnostic redaction.
