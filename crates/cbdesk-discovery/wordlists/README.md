# Short-code word lists

Each file has 2048 words. A short code uses two of them (22 bits) plus a public
slot number. The words are the PAKE password once that exchange exists. They
are not published.

## English

`en.txt` is the BIP-39 English wordlist, copied as published (first word
`abandon`, last word `zoo`). The BIP is under the MIT License. See
`THIRD_PARTY_LICENSES.md`.

## Turkish

`tr.txt` started from the MIT-licensed Turkish list maintained by BlockSight
OÜ. Eight entries were replaced because they collided with another entry after
this fold: `ı→i`, `ö→o`, `ü→u`, `ç→c`, `ş→s`, `ğ→g`.

| Removed | Replacement |
| --- | --- |
| açı | çınar |
| sınır | şelale |
| kaş | leylek |
| ön | palamut |
| dış | ıspanak |
| şu | lodos |
| üç | poyraz |
| âlem | manzara |

Display casing is Turkish-specific: `i` becomes `İ`, and `ı` becomes `I`.
Parsing uses the matching lowercase map.

## Not filtered yet

Pairs at edit distance 1 are still in both lists (on the order of several
hundred pairs). Removing them would shrink a list below 2048 and change the
code length. That filter is deferred to the short-code milestone. Do not treat
a spoken code as strong against a nearby guess; the host must rate-limit
attempts, which the session crate does for authentication failures.
