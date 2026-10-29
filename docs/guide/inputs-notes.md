## Expected result and explanation

```text
Script: printf 'hello'
Lifetime: 5 minutes
Empty script rejected
```

`ShellScript::new` checks text before it can reach execution. `SandboxTtl::minutes`
makes the unit explicit. `?` returns a failure to the caller. The match handles
an empty script as a normal result.

Try changing `minutes(5)` to `minutes(0)`. The program returns a validation error
before printing. Zero does not mean “unlimited.” Restore the positive value.

If the build fails, check the pinned Rust version. No credential can affect this
local lesson. Next: [configure a client](crate::guide::configuration).
