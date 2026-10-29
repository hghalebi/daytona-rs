## What to notice

The handler saves successful output even when cleanup fails. A write failure
retains the cleanup failure too. Replace the destination writer with a file in
your application. A file already downloaded before cleanup survives a cleanup
error. Do not call into_result first if you need that successful output.

The callback deadline limits waiting. It does not prove that the remote command
stopped. The scope still attempts cleanup when the callback returns an error.

Exercise: trace BothFailed and OutputAndCleanup. Each retains both failures;
a source() chain alone follows only one. Next: [restart recovery](crate::guide::recovery).
