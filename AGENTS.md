## Agent skills

### Issue tracker

GitHub Issues in kttq/alba. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context. Before exploring, read `GLOSSARY.md` and the ADRs in `docs/adr/` that touch the area. If they don't exist, proceed silently; `grilling` creates them when a term or decision is resolved. Use the glossary's vocabulary, and flag any conflict with an ADR explicitly rather than silently overriding it.

### Testing

How this project tests: `docs/testing.md`.

### Coding standards

Review judgement calls for comments, names, and code shape: `CODING_STANDARDS.md`.

## App output

`alba --log` prints the log of the running alba, or of the last run. Read it when alba misbehaves.

## Commits

Write each commit subject as one Conventional Commits line of at most 72 characters, as in `feat(scope): summary (#12)`. Add a body only to explain why.
