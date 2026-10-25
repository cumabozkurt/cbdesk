# GitHub settings still required

The workflow in `.github/workflows/ci.yml` can run as soon as it is on the
default branch or on a pull request. The settings below are owner actions.
They are not done.

- Create or confirm the GitHub organization described in the master plan, if
  the repository is meant to leave the personal account.
- Branch protection on `main`: require the CI workflow, dismiss stale reviews,
  and forbid force-pushes. Direct pushes are still possible until this is on.
- Require a `Signed-off-by` trailer via the DCO GitHub App. `tools/check-dco.sh`
  already fails CI when the trailer is missing. The App is a second layer.
- Turn on private vulnerability reporting. `SECURITY.md` points at it.
- Turn on Discussions when the project wants a support channel that is not an
  issue tracker.
- Add a second maintainer before requiring two reviews on `crates/cbdesk-crypto`
  and `crates/cbdesk-net`. `CODEOWNERS` currently lists only the repository
  owner, because a team that does not exist would fail the ruleset.
- Do not enable a public release, a package registry, or a hosted runner with
  secrets for this milestone. There is nothing to ship.

## Status on 2026-10-25 (done through the GitHub API)

- Done: PR #1 merged into `main` with a merge commit (history kept, branch
  `cursor/m0-foundation-0838` kept).
- Done: Discussions on; repository description and topics (remote-desktop,
  rust, flutter, quic, p2p, privacy, agpl); vulnerability alerts, Dependabot
  security updates, private vulnerability reporting, secret scanning and push
  protection on.
- Done: branch protection on `main`: pull request required (0 approvals,
  because there is one maintainer; stale reviews dismissed), the seven CI
  checks required by name, force-push and deletion forbidden. Admin bypass is
  on (`enforce_admins` false) so the owner is not locked out.
- DCO: `tools/check-dco.sh` runs in the `fmt, policy, supply chain` job, so
  the check is already a required status. Commits need `git commit -s`. A
  commit made in the GitHub web editor has no trailer and fails that check
  (the owner can bypass as admin). The DCO GitHub App is not installed (it
  cannot be installed through the API); it is optional second layer.
- Not done (needs the owner): organization creation, second maintainer,
  CODEOWNERS team. See `docs/dev/owner-todo.md`.
