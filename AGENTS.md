## GitHub account

Always use the **geox123** GitHub account in this repo — never the `johnjohto` account, even though it is the active `gh` account.

- For `gh` commands: prefix with `GH_TOKEN=$(gh auth token --user geox123)`, e.g. `GH_TOKEN=$(gh auth token --user geox123) gh issue list`.
- The remote repo is `geox123/minigames` (private): `https://github.com/geox123/minigames`.

## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical labels, names unchanged: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
