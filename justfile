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

# validate the repository's Agent Skill and check skills CLI discovery
agent-skill:
    npx --yes skills-ref@latest validate skills/ssubmit
    npx --yes skills@latest add . --skill ssubmit --list --full-depth
