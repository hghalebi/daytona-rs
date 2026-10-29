# Transfer small files

Start with [upload](crate::guide::files::upload), then [download](crate::guide::files::download). Each lesson creates its own
sandbox. You can run either one without preserving a previous remote resource.

Transfers are bounded in memory. The default limit is 16 MiB. Configure it through
[`ClientConfig::transfer_limit`](crate::ClientConfig::transfer_limit).

Upload overwrites its explicit remote destination. An uncertain response may mean
the write happened. Download creates a new local file and never overwrites one.
A failed or cancelled local write can leave a partial file. Path validation does
not prove that a file exists or that you have permission to access it.
