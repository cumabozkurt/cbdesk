# Third-party licenses

Rust crate licenses are enforced by `cargo-deny` (`deny.toml`), not by copying
every upstream license into this file. The allow-list is MIT, Apache-2.0,
Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib,
MPL-2.0, Unicode-DFS-2016, Unicode-3.0, CC0-1.0, CDLA-Permissive-2.0,
AGPL-3.0-or-later, and GPL-3.0-or-later. CDLA-Permissive-2.0 covers the CA
certificate bundle in `webpki-roots`, which arrives through iroh.

Run `cargo deny check` to see the exact crate versions in `Cargo.lock`.

## Word lists

These files are source data, not crates.

### English

`crates/cbdesk-discovery/wordlists/en.txt` is the BIP-39 English wordlist.
The BIP states "License: MIT". Upstream: the bitcoin/bips `bip-0039` wordlist.

```text
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Turkish

`crates/cbdesk-discovery/wordlists/tr.txt` is derived from the Turkish list in
`osem23/bip39-wordlists-tzur` (`wordlists/tzur-original/turkish.txt`),
copyright BlockSight OÜ, MIT License. Eight words were replaced so that folding
Turkish diacritics does not make two entries identical. The MIT notice above
applies to that list as well. The replacement and the known edit-distance gap
are described in `crates/cbdesk-discovery/wordlists/README.md`.
