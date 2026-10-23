Resumable file transfer into a jailed directory.

`file_write` is high-risk and is not in the unattended grant. A transfer
writes `name.cbdesk-part` and `name.cbdesk-meta` until the root BLAKE3
matches, then renames the file and deletes the sidecar. An interrupted
transfer accepts the missing chunks. zstd is not linked, so compressed
chunks are refused. The CLI reads at most 32 MiB per file.
