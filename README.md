# ssubmit

[![Rust CI](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml/badge.svg)](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml)
[![Crates.io](https://img.shields.io/crates/v/ssubmit.svg)](https://crates.io/crates/ssubmit)

Submit Slurm batch jobs without writing an `sbatch` script.

- [Motivation](#motivation)
- [Install](#install)
- [Usage](#usage)
- [Agent workflows](#agent-workflows)
- [Full usage](#full-usage)

## Motivation

`ssubmit` wraps `sbatch` so a command, its resources and its output settings can
be submitted directly from a shell. Human-readable invocations remain the
default. JSON output and the bundled Agent Skill provide a stable interface for
agents running locally on a Slurm cluster.

## Install

### Release installer

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/mbhall88/ssubmit/main/install/install.sh | sh
# or with wget
wget -qO- https://raw.githubusercontent.com/mbhall88/ssubmit/main/install/install.sh | sh
```

Pass `--yes` to skip the installer confirmation prompt. The installer downloads
the latest release binary; it does not install the Agent Skill.

### Cargo

```shell
cargo install ssubmit
```

### Conda

```shell
conda install -c conda-forge ssubmit
```

### Build from source

```shell
git clone https://github.com/mbhall88/ssubmit.git
cd ssubmit
cargo build --release
target/release/ssubmit --help
```

## Usage

Submit an rsync job named `foo` with 350 MB of memory and a one-week time limit:

```shell
ssubmit --mem 350m --time 1w foo 'rsync -az src/ dest/'
```

Request eight CPUs with the existing passthrough syntax:

```shell
ssubmit --mem 16g --time 1d align \
  'minimap2 -t 8 ref.fa query.fq > out.paf' -- --cpus-per-task=8
```

`NAME` is the Slurm job name. `COMMAND` is the command executed by a batch job
and must be quoted. Options after `--` are passed directly to `sbatch`, so
site-specific Slurm options remain available without new `ssubmit` flags.

### Interactive jobs

Interactive jobs use `salloc` and remain a human-oriented mode:

```shell
ssubmit --interactive --mem 5G --time 8h interactiveJob
ssubmit --interactive --mem 16G --time 4h DevSession \
  --shell bash -- --partition=general --qos=normal
```

Interactive jobs are not supported with `--json`.

### Memory and time

`--mem` accepts values such as `0.5g`, `4.3kb` and `9000`; values are converted
to the Slurm memory format. `--time` accepts values such as `5d`, `10h` and
`45m21s`, as well as Slurm's native time format. Set defaults with
`SSUBMIT_MEMORY` and `SSUBMIT_TIME`.

### Environment export

`ssubmit` preserves its existing default of `--export=ALL`. That can pass tokens,
API keys and other environment variables into a job. Use a narrower export when
the job does not need the full environment:

```shell
ssubmit --export NONE --mem 2G analysis 'python script.py'
ssubmit --export 'PATH,HOME,USER,PYTHONPATH' analysis 'python script.py'
```

The value is passed directly to `sbatch`. No recurring warning is emitted; this
security consideration is documented here and in the Agent Skill.

### Dry runs

`--dry-run` prints the effective Slurm command and generated script without
invoking Slurm:

```shell
ssubmit --dry-run --mem 4G --time 1d dry 'rsync -az src/ dest/' -- --cpus-per-task=8
```

## Agent workflows

The Agent Skill is for an agent running on the same Slurm login or submission
node as the local `ssubmit` and `sbatch` executables. It supports batch planning,
scheduler validation and authorised submission. It does not add SSH, remote
execution, monitoring, cancellation, an MCP server or interactive JSON jobs.

### Install and update the skill

The open `skills` CLI can discover this repository and install the `ssubmit` skill
for compatible harnesses:

```shell
npx skills@latest add mbhall88/ssubmit --skill ssubmit --agent '*' --global --yes
npx skills@latest update ssubmit --global --yes
```

Where a harness supports explicit skill invocation, invoke the skill by the name
`ssubmit`. Slash-command syntax is harness-specific and is not promised by this
repository. Automatic discovery and explicit invocation are both supported where
the harness provides them.

Installing the skill does not install the `ssubmit` executable. Install the
executable separately with the release installer, Cargo or Conda above.

For a cluster without Node.js or outbound network access, clone or copy this
repository on a connected machine and copy `skills/ssubmit/` into the skill
directory used by the target harness. Keep the directory name and `SKILL.md`
frontmatter name as `ssubmit`. Repeat the copy from a newer repository revision
to update it.

The `skills` CLI may report anonymous installation counts for its directory
rankings. They are coarse adoption signals, not active-user, job-submission or
cluster-usage analytics. `ssubmit` adds no first-party telemetry. Set
`DISABLE_TELEMETRY=1` when using the CLI to opt out of its anonymous event.

### JSON planning, testing and submission

Check both executables before starting:

```shell
command -v ssubmit
command -v sbatch
```

Use a dry run when the agent inferred an important command, path, resource or
Slurm option. Show the plan and obtain approval when authority is unclear:

```shell
ssubmit --dry-run --json align \
  'minimap2 -t 8 ref.fa reads.fq > out.paf' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

Use scheduler validation when a submission is not intended:

```shell
ssubmit --test-only --json align \
  'minimap2 -t 8 ref.fa reads.fq' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

Submit only when the user supplied the material parameters and clearly requested
execution:

```shell
ssubmit --json align \
  'minimap2 -t 8 ref.fa reads.fq > out.paf' \
  --mem 16G --time 2h -- --cpus-per-task=8
```

Common passthrough options retain their normal Slurm spelling:

```shell
# partition, GPU, account and QoS values must come from the user or site
ssubmit --json job 'command' -- --partition=<partition>
ssubmit --json job 'command' -- --gres=gpu:1
ssubmit --json job 'command' -- --account=<account>
ssubmit --json job 'command' -- --qos=<qos>
```

Never invent a partition, account or QoS. Preserve the user's command, paths and
resource requests.

### JSON contract

JSON mode emits exactly one JSON object on stdout. Logs and diagnostics are on
stderr. Every failure has a non-zero `ssubmit` exit status. The current contract
is [JSON Schema version 1](schemas/ssubmit-output-v1.schema.json), and every
response has `schema_version: 1`.

| Operation | Invocation | Successful response |
| --- | --- | --- |
| `plan` | `--dry-run --json` | `plan` with the normalised job and effective `sbatch` invocation |
| `test` | `--test-only --json` | `plan` plus scheduler feedback in `test.stdout` and `test.stderr` |
| `submit` | `--json` | `plan` plus `submission.job_id` and optional `submission.cluster` |

On failure, inspect `error.kind`, `error.message`, and any `error.exit_code` or
`error.stderr`. The kinds distinguish validation, process, Slurm and output
parsing failures. Parse the JSON response rather than scraping log lines.

The submission response keeps the job identifier as a string. Report the output
and error patterns from `plan.job.output` and `plan.job.error` (`%x.out` and
`%x.err` by default). JSON submissions use Slurm's parsable output and reject
`--quiet`, which would suppress the identifier; duplicate `--parsable` options
are resolved.

### Cluster smoke test

The release checklist for a real Slurm node is in
[docs/agent-cluster-smoke-test.md](docs/agent-cluster-smoke-test.md). CI uses a
fake `sbatch` executable and does not require a cluster.

## Full usage

The block below is generated from the authoritative Clap definition with
`cargo run --quiet -- --help`. Update it whenever the CLI help changes.

```text
Submit sbatch jobs without having to create a submission script

-----------
# EXAMPLES
-----------

Submit a simple rsync command with a 600MB memory limit.

$ ssubmit -m 600m rsync_my_data "rsync -az src/ dest/"

Submit a command that involves piping the output into another command. sbatch options
are passed after a `--`.

$ ssubmit -m 4G align "minimap2 -t 8 ref.fa reads.fq | samtools sort -o sorted.bam" -- -c 8

Start an interactive session with 5GB memory for 8 hours.

$ ssubmit --interactive -m 5G -t 8h interactiveJob

Start an interactive session with custom shell and additional SLURM options.

$ ssubmit --interactive -m 16G -t 4h DevSession --shell bash -- --partition=general --qos=normal

Usage: ssubmit [OPTIONS] <NAME> [COMMAND] [-- <REMAINDER>...]

Arguments:
  <NAME>
          Name of the job

          See `man sbatch | grep -A 2 'job-name='` for more details.

  [COMMAND]
          Command to be executed by the job

          For batch jobs, this is required. For interactive jobs (--interactive), this is optional and defaults to starting a shell session.

  [REMAINDER]...
          Options to be passed on to sbatch or salloc (for interactive jobs)

Options:
  -o, --output <OUTPUT>
          File to write job stdout to. (See `man sbatch | grep -A 3 'output='`)

          Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.

          [default: %x.out]

  -e, --error <ERROR>
          File to write job stderr to. (See `man sbatch | grep -A 3 'error='`)

          Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.

          [default: %x.err]

  -m, --mem <size[unit]>
          Specify the real memory required per node. e.g., 4.3kb, 7 Gb, 9000, 4.1MB become 5KB, 7000M, 9000M, and 5M, respectively.

          If no unit is specified, megabytes will be used, as per the sbatch default. The value will be rounded up to the nearest megabyte. If the value is less than 1M, it will be rounded up to the nearest kilobyte. See `man sbatch | grep -A 4 'mem='` for the full details.

          [env: SSUBMIT_MEMORY=]
          [default: 1G]

  -t, --time <TIME>
          Time limit for the job. e.g. 5d, 10h, 45m21s (case-insensitive)

          Run `man sbatch | grep -A 7 'time=<'` for more details. If a single digit is passed, it will be passed straight to sbatch (i.e. minutes). However, 5m5 will be considered 5 minutes and 5 seconds.

          [env: SSUBMIT_TIME=]
          [default: 1d]

  -S, --shebang <SHEBANG>
          The shell shebang for the submission script

          [env: SSUBMIT_SHEBANG=]
          [default: "#!/usr/bin/env bash"]

  -s, --set <SET>
          Options for the set command in the shell script

          For example, to exit when the command exits with a non-zero code and to treat unset variables as an error during substitution, pass 'eu'. Pass '' or "" to set nothing

          [env: SSUBMIT_SET=]
          [default: "euxo pipefail"]

  -n, --dry-run
          Print the sbatch command and submission script that would be executed, but do not execute them

      --json
          Print a versioned machine-readable response. Dry runs return a plan, --test-only returns scheduler feedback, and batch submissions return a Slurm job identifier. JSON mode does not support interactive jobs

  -T, --test-only
          Return an estimate of when the job would be scheduled to run given the current queue. No job is actually submitted. [sbatch --test-only]

  -i, --interactive
          Request an interactive job session instead of a batch job

          This will use `salloc` instead of `sbatch` and automatically start an interactive shell. The command argument becomes optional and defaults to the user's shell.

      --shell <SHELL>
          Shell to use for interactive sessions

          Only used when --interactive is specified. Defaults to the user's login shell.

          [default: auto]

      --export <EXPORT>
          Control which environment variables are exported to the job

          Passed directly to sbatch as --export=<value>. Use 'NONE' to export no variables, 'ALL' to export all variables, or specify specific variables like 'PATH,HOME'.

          [default: ALL]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
