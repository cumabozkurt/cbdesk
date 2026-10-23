Keyboard and pointer injection.

`inject_os` moves the real pointer and is only for a host that already
called `authorize(Input)`. Tests use `InputLog` and do not call `inject`.
There is no flag that injects without that grant.
