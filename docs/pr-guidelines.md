# Pull request title and description

## Agent handoff

If you were given this file path:

1. Follow the guideline exactly.
2. In your response message paste the title in a code block `title` and the description in a separate code block `description`. Send the response with only these two separate code blocks in it.
3. Then stop. Do not open a pull request.

---

Grouper uses the format most teams already recognize: **Conventional Commits** for the title, and a short structured body.

## Title

```
<type>(<optional-scope>): <imperative summary>
```

Rules:

- Use lowercase `type`; optional `scope` in parentheses; colon and space; then a short imperative summary (no trailing period).
- Keep the whole title around **72 characters** or less.
- Prefer one clear change; if the PR mixes concerns, the title names the primary intent.

| Type | Use when |
|------|----------|
| `feat` | User-facing feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `chore` | Tooling, hygiene, config with no product behavior change |
| `refactor` | Internal restructuring, same behavior |
| `test` | Tests only |
| `ci` | CI configuration |
| `perf` | Performance improvement |

Examples:

- `docs(S0-01): add AGENTS.md and repo hygiene`
- `feat(flyout): open group flyout from taskbar shortcut`
- `fix(icons): free HBITMAP after Shell extraction`

## Description

Use this Markdown body (omit a section only if it truly does not apply):

```markdown
## Summary
- <what changed and why — 1–3 bullets>

## Test plan
- [ ] <command or manual step that proves it>
- [ ] <edge case or Windows smoke step if relevant>
```

Optional trailing sections when useful: **Breaking changes**, **Notes** (unverified items, follow-ups). Do not paste large diffs into the description.

## Checklist before opening

1. Branch name matches `task/<ID>-<slug>`.
2. Title and body follow this doc.
3. CI on `windows-latest` is expected to pass (or gaps called out under Notes).
4. Agent posts a review summary and waits for Preston before opening the PR; only Preston merges.
