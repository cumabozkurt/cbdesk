#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Turkish and English catalogs must contain the same non-empty keys.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
python3 - "$root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
en = json.loads((root / "app/i18n/en.json").read_text(encoding="utf-8"))
tr = json.loads((root / "app/i18n/tr.json").read_text(encoding="utf-8"))
if set(en) != set(tr):
    missing_tr = sorted(set(en) - set(tr))
    missing_en = sorted(set(tr) - set(en))
    raise SystemExit(f"catalog keys differ\ntr missing {missing_tr}\nen missing {missing_en}")
empty = [key for key, value in {**en, **tr}.items() if not str(value).strip()]
if empty:
    raise SystemExit(f"empty catalog values: {empty}")
print(f"i18n: {len(en)} keys match")
PY
