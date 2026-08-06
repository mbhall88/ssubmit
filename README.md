# ssubmit

[![Rust CI](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml/badge.svg)](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml)
[![Crates.io](https://img.shields.io/crates/v/ssubmit.svg)](https://crates.io/crates/ssubmit)

Submit sbatch jobs without having to create a submission script

- [Motivation](#motivation)
- [Install](#install)
- [Usage](#usage)
- [Agent workflows](#agent-workflows)
- [Full usage](#full-usage)

## Motivation

This project is motivated by the fact that I want to just be able to submit commands as
jobs and I don't want to fluff around with making a submission script.

`ssubmit` wraps that whole process and lets you live your best lyf #blessed.

Human-readable invocations remain the default. JSON output and the bundled Agent
Skill provide a stable interface for agents running locally on a Slurm cluster.

## Install

### Release installer

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/mbhall88/ssubmit/main/install/install.sh | sh
# or with wget
wget -qO- https://raw.githubusercontent.com/mbhall88/ssubmit/main/install/install.sh | sh
```

The installer downloads the latest release binary; it does not install the Agent
Skill.

> [!NOTE]
> Releases are moving to a generated `ssubmit-installer.sh` published as a
> release asset. This script keeps working in the meantime; see
> [docs/releasing.md](docs/releasing.md) for the transition.

You can pass options to the script like so

```
$ curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/mbhall88/ssubmit/main/install/install.sh | sh -s -- --help
install.sh [option]

Fetch and install the latest version of ssubmit, if ssubmit is already
installed it will be updated to the latest version.

Options
        -V, --verbose
                Enable verbose output for the installer

        -f, -y, --force, --yes
                Skip the confirmation prompt during installation

        -p, --platform
                Override the platform identified by the installer [default: apple-darwin]

        -b, --bin-dir
                Override the bin installation directory [default: /usr/local/bin]

        -a, --arch
                Override the architecture identified by the installer [default: x86_64]

        -B, --base-url
                Override the base URL used for downloading releases [default: https://github.com/mbhall88/ssubmit/releases]

        -h, --help
                Display this help message
```

### Cargo

![Crates.io Version](https://img.shields.io/crates/v/ssubmit)
![Crates.io Total Downloads](https://img.shields.io/crates/d/ssubmit)

```shell
$ cargo install ssubmit
```

### Conda

![Conda Version](https://img.shields.io/conda/v/conda-forge/ssubmit)
![Conda Downloads](https://img.shields.io/conda/d/conda-forge/ssubmit)

```shell
$ conda install -c conda-forge ssubmit
```

### Build from source

```shell
$ git clone https://github.com/mbhall88/ssubmit.git
$ cd ssubmit
$ cargo build --release
$ target/release/ssubmit -h
```

## Usage

Submit an rsync job named "foo" and request 350MB of memory and a one week time limit

```shell
$ ssubmit -m 350m -t 1w foo "rsync -az src/ dest/"
```

Submit a job that needs 8 CPUs

```shell
$ ssubmit -m 16g -t 1d align "minimap2 -t 8 ref.fa query.fq > out.paf" -- -c 8
```

Start an interactive session with 5GB memory for 8 hours

```shell
$ ssubmit --interactive -m 5G -t 8h interactiveJob
```

Start an interactive session with custom shell and additional SLURM options

```shell
$ ssubmit --interactive -m 16G -t 4h DevSession --shell bash -- --partition=general --qos=normal
```

The basic anatomy of a `ssubmit` call is

```
ssubmit [OPTIONS] <NAME> [COMMAND] [-- <REMAINDER>...]
```

`NAME` is the name of the job (the `--job-name` parameter in `sbatch` or `salloc`).

`COMMAND` is what you want to be executed by the job. For batch jobs, it **must** be quoted (single or
double) and is required. For interactive jobs (`--interactive`), this is optional and defaults to starting an interactive shell session.

`REMAINDER` is any (optional) [`sbatch`-specific options](https://slurm.schedmd.com/sbatch.html#lbAG) (for batch jobs) or [`salloc`-specific options](https://slurm.schedmd.com/salloc.html) (for interactive jobs) you want to pass on. These
must follow a `--` after `COMMAND` (or after `NAME` if no command is provided for interactive jobs).

### Interactive jobs

You can start interactive job sessions using the `--interactive` (or `-i`) flag. This uses `salloc` instead of `sbatch` and automatically starts an interactive shell session.

```shell
# Start an interactive session with default shell
$ ssubmit --interactive -m 8G -t 4h my_session

# Start an interactive session with a specific shell
$ ssubmit --interactive --shell bash -m 16G -t 8h dev_work

# Start an interactive session with additional SLURM options
$ ssubmit --interactive -m 32G -t 12h gpu_session -- --partition=gpu --gres=gpu:1
```

When using `--interactive`:

- The command argument is optional and defaults to starting an interactive shell
- If no command is provided, `ssubmit` will automatically detect your current shell and start an interactive session
- You can specify a different shell using the `--shell` option
- All the same memory and time parsing features work just like with batch jobs
- Additional Slurm options can be passed after `--` just like with batch jobs

Interactive jobs are not supported with `--json`.

### Memory

Memory (`-m,--mem`) is intended to be a little more user-friendly than the `sbatch
--mem` option. For example, you can pass `-m 0.5g` and `ssubmit` will interpret and
convert this as 500M. Units are case-insensitive. Memory values over 1M will be rounded up to the nearest whole number.
For example, 1.1M will be rounded up to 2M. If you want to use the default memory limit of your cluster, then just pass
`-m 0`.

For simplicity's sake, all values over one megabyte are passed to sbatch as megabytes - e.g., 1.1G will be passed as 1100M.

The environment variable `SSUBMIT_MEMORY` can be set to a default memory limit. This can be overridden by passing `-m`.

### Time

As with memory, time (`-t,--time`) is intended to be simple. If you want a time limit of
three days, then just pass `-t 3d`. Want two and a half hours? Then `-t 2h30m` works. If
you want to just use the default limit of your cluster, then just pass `-t 0`. You can
also just pass the [time format `sbatch` uses](https://slurm.schedmd.com/sbatch.html#OPT_time) and this will be seamlessly passed on. For
a full list of supported time units, check out the
[`duration-str`](https://github.com/baoyachi/duration-str) repo. One thing to note is that passing a single digit, without a unit, will be interpreted by
slurm as minutes. However, not providing a unit in the example of `5m3` will be interpreted as 5 minutes and 3 seconds.

The environment variable `SSUBMIT_TIME` can be set to a default time limit. This can be overridden by passing `-t`.

### Environment export

By default, `ssubmit` exports all environment variables to the job using `--export=ALL`. This ensures that your job has access to the same environment as your current shell session.

You can control which environment variables are exported using the `--export` option:

```shell
# Export all environment variables (default behavior)
$ ssubmit -m 2g analysis "python script.py"

# Export no environment variables
$ ssubmit --export NONE -m 2g analysis "python script.py"

# Export specific environment variables only
$ ssubmit --export "PATH,HOME,USER,PYTHONPATH" -m 2g analysis "python script.py"
```

This option is passed directly to `sbatch` as `--export=<value>`, so it supports all the same values as the sbatch `--export` option. Common values include:
- `ALL` - Export all environment variables (default)
- `NONE` - Export no environment variables
- Comma-separated list (e.g., `PATH,HOME`) - Export only the specified variables

Note that if you specify `--export` in the remainder arguments (after `--`), it will override the default `--export=ALL` behavior.

Because the default exports everything, a job can inherit tokens, API keys and
other secrets from your shell. Use a narrower export when the job does not need
the full environment. No recurring warning is emitted; this security
consideration is documented here and in the Agent Skill.

### Dry run

You can see what `ssubmit` would do without actually submitting a job using dry run
(`-n,--dry-run`). This will print the `sbatch` command (for batch jobs) or `salloc` command (for interactive jobs) that would have been executed.

For batch jobs, it also shows the submission script:

```shell
$ ssubmit -n -m 4g -t 1d dry "rsync -az src/ dest/" -- -c 8
[2022-01-19T08:58:58Z INFO  ssubmit] Dry run requested. Nothing submitted
sbatch --export=ALL -c 8 <script>
=====<script>=====
#!/usr/bin/env bash
#SBATCH --job-name=dry
#SBATCH --mem=4000M
#SBATCH --time=24:0:0
#SBATCH --error=%x.err
#SBATCH --output=%x.out
set -euxo pipefail

rsync -az src/ dest/
=====<script>=====
```

For interactive jobs, it shows the `salloc` command:

```shell
$ ssubmit --interactive -n -m 8G -t 4h my_session
[2022-01-19T08:58:58Z INFO  ssubmit] Dry run requested. Nothing submitted
salloc --job-name my_session --mem 8000M --time 4:0:0 srun --pty zsh -l
```

### Script settings

The default shebang for the script is `#!/usr/bin/env bash`. However, if you'd prefer
something else, pass this with `-S,--shebang` or set the environment variable `SSUBMIT_SHEBANG`.

Additionally, we use `set -euxo pipefail` by default, which will exit when a command exits with a
non-zero exit code (`e`), error when trying to use an unset variable (`u`), print
all commands that were run to stderr (`x`), and exit if a command in a pipeline fails
(`-o pipefail`). You can change these setting with `-s,--set` or the environment variable `SSUBMIT_SET`. You can turn this off
by passing `-s ''`.

### Log files

By default, the stderr and stdout of the job are sent to `%x.err` and `%x.out`,
respectively. `%x` is a filename pattern for job name. So if the job name is foo, the
stderr file will be `foo.err`. You can see all available patterns in
[the docs](https://slurm.schedmd.com/sbatch.html#SECTION_%3CB%3Efilename-pattern%3C/B%3E).
You don't have to use patterns of course.

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

## Full usage

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
