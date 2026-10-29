# Save intent and recover after interruption

Use a versioned application record to keep a creation reference with its trusted
context. A reference is correlation data, not permission or proof of creation.

## Before you start

Use the first-command dependencies, plus serde with its derive feature and
serde_json. Set DAYTONA_API_KEY and DAYTONA_ORGANIZATION_ID for the same account.
This lesson supports the Daytona cloud endpoints only. A saved file cannot choose
another endpoint. The organization value is a local guard, not remote verification
of the credential's membership. Confirm the account in your provider settings.

Create a new record before a live, billable scope runs:

```sh
mkdir -p .plan
cargo run --example recover_creation -- .plan/recovery.json --start --run-live
```

The file must not already exist. Failure to save it prevents creation. After an
interruption, read the same record without creating or deleting anything:

```sh
cargo run --example recover_creation -- .plan/recovery.json
```

After checking the account, workflow, and found identity, request deletion explicitly:

```sh
cargo run --example recover_creation -- .plan/recovery.json --delete YOUR_SANDBOX_ID
```

The identity must match the reconciled receipt. A matching label alone is not
authorization. This record format is specific to the lesson; it does not replace
the SDK CreationReference format or accept older bare-reference files.

## Complete example
