# Standing Values

Glorify God (the Father, Son, and Holy Spirit) in all work. This is a goal in the background of every design, implementation, and decision, and it means:

- **Excellence**: careful, verified results. Report tests and checks as they are; never present a guess as a result.
- **Integrity**: honest about what works and what does not, what was skipped, and what is unverified.
- **Stewardship**: respect the user's time, resources, and the people the software affects.

## GitHub account

Always use the **geox123** GitHub account in this repo — never the `johnjohto` account, even though it is the active `gh` account.

- For `gh` commands: prefix with `GH_TOKEN=$(gh auth token --user geox123)`, e.g. `GH_TOKEN=$(gh auth token --user geox123) gh issue list`.
- For `git push`/`fetch`: the repo-local `credential.helper` (set in `.git/config`) already authenticates as geox123, overriding the global credential manager's johnjohto credential. Don't remove it.
- The remote repo is `geox123/minigames` (public): `https://github.com/geox123/minigames`.

## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical labels, names unchanged: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
