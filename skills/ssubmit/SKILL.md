---
name: ssubmit
description: Submit Slurm batch jobs from a local login or submission node with ssubmit and sbatch. Use when planning resources, validating a scheduler request, or submitting a batch command with JSON output.
license: MIT
compatibility: Requires an ssubmit release with JSON mode, local sbatch access, and execution on a Slurm login or submission node.
metadata:
  contract: "ssubmit JSON schema v1"
  version: "1"
---

# ssubmit

Use this skill for local Slurm batch planning, scheduler validation and authorised
submission. The skill does not provide remote execution, SSH, monitoring,
cancellation or interactive JSON jobs.

## Check the execution environment

Run these checks on the node where the job should be submitted:

```sh
command -v ssubmit
command -v sbatch
ssubmit --version
```

If either executable is missing, explain which one is unavailable. Do not replace
local execution with SSH or another remote service.

## Preserve the request

Keep the user's command, paths, job name and requested resources unchanged.
Ask for missing material details rather than guessing them. In particular, never
invent a site-specific partition, account or QoS. Pass those options only when
the user supplied them or confirmed them:

```sh
# Eight CPUs
ssubmit --json align 'minimap2 -t 8 ref.fa reads.fq' -- --cpus-per-task=8

# A user-supplied partition
ssubmit --json align 'minimap2 ref.fa reads.fq' -- --partition=<partition>

# A user-supplied GPU request
ssubmit --json train 'python train.py' -- --gres=gpu:1

# A user-supplied account and QoS
ssubmit --json report 'python report.py' -- --account=<account> --qos=<qos>
```

The options after `--` are passed through to `sbatch`. Use the option spelling
and values the user gave you. Do not turn a site-specific passthrough option into
a new first-class `ssubmit` option.

## Choose planning or submission

Use `--dry-run --json` when you inferred an important command, path, resource or
Slurm option, or when the user has not clearly authorised submission. Show the
plan and ask for confirmation before submitting it:

```sh
ssubmit --dry-run --json align \
  'minimap2 -t 8 ref.fa reads.fq > out.paf' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

The dry run never invokes `sbatch`. A successful response has
`operation: "plan"`, `ok: true` and a `plan` object.

Submit directly only when the user supplied the material command, paths,
resources and site-specific options and clearly requested execution:

```sh
ssubmit --json align \
  'minimap2 -t 8 ref.fa reads.fq > out.paf' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

A successful submission has `operation: "submit"`, `ok: true` and a
`submission` object. Report `submission.job_id` exactly as a string and report
`submission.cluster` when it is not null. Also report the output and error
patterns from `plan.job.output` and `plan.job.error` when the response includes a
plan. The defaults are `%x.out` and `%x.err`, where `%x` is the job name.

Use scheduler validation when it is useful and submission is not intended:

```sh
ssubmit --test-only --json align \
  'minimap2 -t 8 ref.fa reads.fq' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

This invokes `sbatch --test-only` and does not submit a job. A successful
response has `operation: "test"`, `ok: true`, a `plan` and a `test` object.
Use `test.stdout` and `test.stderr` as scheduler feedback; do not scrape human
logs for a job identifier.

JSON mode does not support `--interactive`. Do not combine those options. Job
monitoring, completed-job inspection, log retrieval and cancellation are outside
this skill; use the site's normal Slurm commands only when the user explicitly
asks for a separate, human-oriented workflow.

## Parse the machine contract

The current contract is JSON schema version `1`. Every invocation in JSON mode
writes exactly one JSON object to stdout. Logs and diagnostics are written to
stderr. Parse stdout as JSON and inspect `schema_version`, `operation` and `ok`:

- `ok: true` contains the result for `plan`, `test` or `submit`.
- `ok: false` contains `error.kind`, `error.message` and, when available,
  `error.exit_code` and `error.stderr`.
- `error.kind` distinguishes validation, process, Slurm and output-parsing
  failures. Every failure has a non-zero `ssubmit` exit status.

Do not treat a non-zero exit as a successful submission, even if a diagnostic
mentions a job. Do not parse logger lines or human-mode output when JSON mode is
available. The committed schema is at
`schemas/ssubmit-output-v1.schema.json` in the ssubmit repository.

`ssubmit` adds `--parsable` for JSON submissions so the job identifier is
machine-readable. Do not add `--quiet` in JSON mode because it suppresses the
identifier. A user-supplied `--parsable` is safe and is de-duplicated.

## Environment export

The existing default is `--export=ALL`. It can copy credentials, API keys and
other agent environment variables into the job. Preserve that default unless the
user asks for a narrower export, and do not add a recurring warning to command
output. Safer alternatives are:

```sh
ssubmit --json job 'python job.py' --export NONE
ssubmit --json job 'python job.py' --export 'PATH,HOME,USER'
```

The value is passed through to `sbatch`; follow its documented export syntax.

## Installation and explicit use

Compatible harnesses can discover this skill from the repository. Where the
harness supports explicit skill invocation, ask the user to invoke `ssubmit` by
name. Slash-command syntax is harness-specific and is not universal.

Install or update the skill with the open `skills` CLI:

```sh
npx skills@latest add mbhall88/ssubmit --skill ssubmit --agent '*' --global --yes
npx skills@latest update ssubmit --global --yes
```

Installing the skill does not install the `ssubmit` executable. Install the
executable separately with the project's release installer or `cargo install
ssubmit`.

On a cluster without Node.js or outbound network access, clone or copy this
repository on a machine that can reach it, then copy the whole
`skills/ssubmit/` directory into the skill directory used by the target harness.
Keep `SKILL.md` inside a directory named `ssubmit` so the name and directory stay
valid under the Agent Skills specification. Update it by repeating the copy from
the desired repository revision.

The `skills` CLI may report anonymous installation counts. Those counts are
coarse adoption signals, not active-user, job-submission or cluster-usage
analytics. `ssubmit` adds no first-party telemetry. Set `DISABLE_TELEMETRY=1`
when using the CLI if its anonymous event is not appropriate for your
environment.
