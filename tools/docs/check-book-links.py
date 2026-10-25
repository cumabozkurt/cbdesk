#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check links in the built user guide. Local targets must exist."""

import sys
import urllib.error
import urllib.request
from html.parser import HTMLParser
from pathlib import Path


class Links(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.found: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        data = {key: value for key, value in attrs if value is not None}
        if tag == "a" and "href" in data:
            self.found.append(data["href"])
        if tag in {"img", "script"} and "src" in data:
            self.found.append(data["src"])
        if tag == "link" and "href" in data:
            self.found.append(data["href"])


def local_target(page: Path, url: str, root: Path) -> Path | None:
    bare = url.split("#", 1)[0].split("?", 1)[0]
    if bare == "" or bare.startswith(("mailto:", "javascript:", "data:")):
        return None
    if bare.startswith(("http://", "https://")):
        return None
    if bare.startswith("/"):
        target = (root / bare.lstrip("/")).resolve()
    else:
        target = (page.parent / bare).resolve()
    if target.is_dir():
        target = target / "index.html"
    try:
        target.relative_to(root)
    except ValueError:
        raise SystemExit(f"link escapes the site: {page} -> {url}")
    return target


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-book-links.py SITE")
    root = Path(sys.argv[1]).resolve()
    pages = sorted(root.rglob("*.html"))
    if not pages:
        raise SystemExit(f"no html under {root}")
    missing: list[str] = []
    external: set[str] = set()
    for page in pages:
        parser = Links()
        parser.feed(page.read_text(encoding="utf-8", errors="replace"))
        for url in parser.found:
            if url.startswith(("http://", "https://")):
                external.add(url.split("#", 1)[0])
                continue
            target = local_target(page, url, root)
            if target is not None and not target.is_file():
                missing.append(f"{page.relative_to(root)} -> {url}")
    if missing:
        print("broken local links:")
        print("\n".join(missing))
        raise SystemExit(1)
    failures: list[str] = []
    for url in sorted(external):
        request = urllib.request.Request(url, method="GET", headers={"User-Agent": "cbdesk-book-link-check"})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                if response.status >= 400:
                    failures.append(f"{response.status} {url}")
        except urllib.error.HTTPError as err:
            failures.append(f"{err.code} {url}")
        except urllib.error.URLError as err:
            failures.append(f"{err.reason} {url}")
    if failures:
        print("broken external links:")
        print("\n".join(failures))
        raise SystemExit(1)
    print(f"links: {len(pages)} pages, {len(external)} external urls")


if __name__ == "__main__":
    main()
