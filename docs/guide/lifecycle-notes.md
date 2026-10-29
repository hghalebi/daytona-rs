## Expected result and explanation

```text
Recovery reference: .plan/manual-<operation-id>.json
manual lifecycle complete
```

The operation ID varies. The reference file is created and synced before submission.
Keep it if execution is interrupted. Validation consumes the draft. Submission
consumes the validated request. Waiting consumes the receipt, but errors retain it.

A readiness error still requires cleanup. The example recovers the receipt, retains
the readiness cause, and attempts deletion. After work begins, it handles work and
cleanup separately. No `?` between work completion and deletion skips cleanup.

Try reading the two cleanup branches before changing code. One handles failed
readiness; the other handles completed work. Prefer a scope when you do not need
manual lifecycle control.

If creation is uncertain, the returned error and saved reference support recovery.
Do not repeat the POST. Next: [reconcile creation](crate::guide::recovery).
