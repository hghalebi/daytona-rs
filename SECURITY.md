# Security reports

Report a suspected vulnerability through
[GitHub's private vulnerability reporting](https://github.com/hghalebi/daytona-rs/security/advisories/new).
Do not disclose credentials, customer data, or exploit details in a public issue.

Include the affected commit or version, a minimal reproduction using synthetic
data, the observed impact, and any mitigation you have identified. No response
time commitment is currently offered for this independently maintained project.

The first crates.io release is still being prepared. Security fixes are developed
on `main`; there are no separately maintained older release branches.

## Relevant boundaries

- Transport credentials are sent only to the caller-configured API and toolbox
  endpoints. Treat those endpoint configurations as trusted input.
- Validated paths and IDs do not establish remote authorization or filesystem
  confinement.
- Cancellation and process exit do not perform asynchronous cleanup. Preserve
  recovery references and account for unfinished remote operations.
- Debug redaction and response limits are tested properties. They are not a
  guarantee about every downstream logging choice or dependency behavior.

See [the README](README.md) and [implementation contract](docs/contracts.md) for
operational details and the scope of the tests.
