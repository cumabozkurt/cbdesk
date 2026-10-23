# Upstream import

There is nothing to sync. This repository was created empty, apart from a
README and the AGPL license, and the code added here was written for the
modular crate layout.

The master plan's first milestone also describes forking RustDesk, keeping that
git history for attribution, and rebranding it. That import has not been done.
ADR-002 is the decision to sequence it later, as a separate change that:

- preserves upstream copyright notices,
- does not claim the current crates are derived from that tree,
- runs `tools/rebrand-check.sh` so the upstream name survives only in
  attribution files,
- and does not copy server addresses, ports 21115–21119, or project URLs into
  the CBDesk defaults.

Until that import exists, this file has no submodule, no remote, and no commit
pin.
