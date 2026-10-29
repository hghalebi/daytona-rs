## Expected result and explanation

```text
Uploaded 6 bytes; sandbox deleted
```

`LocalSourcePath` names a local file. `SandboxPath` names an absolute remote path.
They are different types because swapping them is a mistake. The transfer is
bounded in memory. The receipt reports the number of bytes accepted by the provider.

Try changing the local text and check the byte count. Non-ASCII characters may use
more than one byte. If the input file is missing, the callback fails and the scope
still attempts deletion. A timeout during upload may leave the remote file changed.
Next: [download a file](crate::guide::files::download).
