## Expected result and explanation

```text
Handler compiled; no remote failure was simulated
```

The main function only reports compilation. It does not invoke every handler
branch. The protocol tests exercise success, work failure, cleanup failure, and
combined failure deterministically.

`run_report` retains successful output in `CleanupFailed`. The handler writes that
output and still returns the cleanup error. If writing also fails, its concrete
error retains both failures. `into_result` is convenient when you accept losing
successful output on cleanup failure.

Try tracing the `BothFailed` branch. Which errors survive? Both remain in
`ScopeError::Both`; the standard error source chain shows only one branch.
Next: [manual lifecycle](crate::guide::lifecycle).
