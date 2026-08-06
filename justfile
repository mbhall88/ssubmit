PROJECT := "ssubmit"

# run clippy to check for linting issues
lint:
    cargo clippy --all-features --all-targets -- -D warnings

# run all tests
test:
    cargo test -v --all-targets --no-fail-fast

# get coverage with tarpaulin
coverage:
    cargo tarpaulin -t 300 -- --test-threads 1

# check the generated release workflow is in sync with dist-workspace.toml
dist-check:
    dist generate --check

# show the release artifacts dist would build, without building them
dist-plan:
    dist plan

# validate the repository's Agent Skill and check skills CLI discovery
agent-skill:
    npx --yes skills-ref@latest validate skills/ssubmit
    npx --yes skills@latest add . --skill ssubmit --list --full-depth
