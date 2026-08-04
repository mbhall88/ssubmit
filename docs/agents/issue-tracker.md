# GitHub issue tracker

Issues and PRDs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply or remove labels**: `gh issue edit <number> --add-label "..."` or `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this automatically inside a clone.

## Pull requests as a triage surface

**PRs as a request surface: no.** Set this to `yes` if the repository later treats external PRs as feature requests.

When set to `yes`, PRs use the same labels and states as issues:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>`.
- **List external PRs**: use `gh pr list`, retaining authors with `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR` or `NONE` associations.
- **Comment, label or close**: use the corresponding `gh pr` commands.

GitHub shares one number space across issues and PRs. For a bare `#42`, try `gh pr view 42` and then `gh issue view 42`.

## Publishing to the issue tracker

When a skill says to publish to the issue tracker, create a GitHub issue.

## Fetching a ticket

When a skill says to fetch a ticket, run `gh issue view <number> --comments`.

## Wayfinding operations

The map is one issue with child issues as tickets.

- **Map**: an issue labelled `wayfinder:map`, containing Notes, Decisions-so-far and Fog.
- **Child ticket**: a GitHub sub-issue linked to the map. If sub-issues are unavailable, use a task list and add `Part of #<map>` to the child.
- **Blocking**: use GitHub’s native issue dependencies. Where unavailable, add `Blocked by: #<n>` to the child.
- **Frontier query**: select the first open, unblocked and unassigned child in map order.
- **Claim**: `gh issue edit <number> --add-assignee @me`.
- **Resolve**: comment with the answer, close the child, then add a context pointer to the map’s Decisions-so-far section.
