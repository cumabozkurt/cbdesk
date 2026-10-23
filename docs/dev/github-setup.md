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
