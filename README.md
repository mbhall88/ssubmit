# ssubmit

[![Rust CI](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml/badge.svg)](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml)
[![Crates.io](https://img.shields.io/crates/v/ssubmit.svg)](https://crates.io/crates/ssubmit)

Submit sbatch jobs without having to create a submission script

- [Motivation](#motivation)
- [Install](#install)
- [Usage](#usage)


## Motivation

This project is motivated by the fact that I want to just be able to submit commands as
jobs and I don't want to fluff around with making a submission script.

`ssubmit` wraps that whole process and lets you live your best lyf #blessed.

## Install

**tl;dr**

```shell
curl -sSL install.ssubmit.mbh.sh | sh
# or with wget
wget -nv -O - install.ssubmit.mbh.sh | sh
```

You can pass options to the script like so

```
$ curl -sSL install.ssubmit.mbh.sh | sh -s -- --help
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

```
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

```
$ ssubmit -h
Submit sbatch jobs without having to create a submission script

Usage: ssubmit [OPTIONS] <NAME> [COMMAND] [-- <REMAINDER>...]

Arguments:
  <NAME>            Name of the job
  [COMMAND]         Command to be executed by the job. For batch jobs, this is required. For interactive jobs (--interactive), this is optional and defaults to starting a shell session
  [REMAINDER]...    Options to be passed on to sbatch or salloc (for interactive jobs)

Options:
  -o, --output <OUTPUT>    File to write job stdout to. (See `man sbatch | grep -A 3 'output='`) [default: %x.out]
  -e, --error <ERROR>      File to write job stderr to. (See `man sbatch | grep -A 3 'error='`) [default: %x.err]
  -m, --mem <size[unit]>   Specify the real memory required per node. e.g., 4.3kb, 7 Gb, 9000, 4.1MB become 5KB, 7000M, 9000M, and 5M, respectively [env: SSUBMIT_MEMORY=] [default: 1G]
  -t, --time <TIME>        Time limit for the job. e.g. 5d, 10h, 45m21s (case-insensitive) [env: SSUBMIT_TIME=] [default: 1d]
  -S, --shebang <SHEBANG>  The shell shebang for the submission script [env: SSUBMIT_SHEBANG=] [default: "#!/usr/bin/env bash"]
  -s, --set <SET>          Options for the set command in the shell script [env: SSUBMIT_SET=] [default: "euxo pipefail"]
  -n, --dry-run            Print the sbatch command and submission script that would be executed, but do not execute them
  -T, --test-only          Return an estimate of when the job would be scheduled to run given the current queue. No job is actually submitted. [sbatch --test-only]
  -i, --interactive        Request an interactive job session instead of a batch job
      --shell <SHELL>      Shell to use for interactive sessions [default: auto-detected]
      --export <EXPORT>    Control which environment variables are exported to the job [default: ALL]
  -h, --help               Print help (see more with '--help')
  -V, --version            Print version
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

### Interactive Jobs

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
- Additional SLURM options can be passed after `--` just like with batch jobs

### Memory

Memory (`-m,--mem`) is intended to be a little more user-friendly than the `sbatch
--mem` option. For example, you can pass `-m 0.5g` and `ssubmit` will interpret and
convert this as 500M. Units are case-insensitive. Memory values over 1M will be rounded up to the nearest whole number. 
For example, 1.1M will be rounded up to 2M. If you want to use the default memory limit of your cluster, then just pass 
`-m 0`.

For simplicity's sake, all values over one megabyte are passed to sbatch as megabytes - e.g., 1.1G will be passed as 1100M.

The environment variable `SSUBMIT_MEM` can be set to a default memory limit. This can be overridden by passing `-m`.

### Time

As with memory, time (`-t,--time`) is intended to be simple. If you want a time limit of
three days, then just pass `-t 3d`. Want two and a half hours? Then `-t 2h30m` works. If
you want to just use the default limit of your cluster, then just pass `-t 0`. You can
also just pass the [time format `sbatch` uses](https://slurm.schedmd.com/sbatch.html#OPT_time) and this will be seamlessly passed on. For
a full list of supported time units, check out the
[`duration-str`](https://github.com/baoyachi/duration-str) repo. One thing to note is that passing a single digit, without a unit, will be interpreted by 
slurm as minutes. However, not providing a unit in the example of `5m3` will be interpreted as 5 minutes and 3 seconds.

The environment variable `SSUBMIT_TIME` can be set to a default time limit. This can be overridden by passing `-t`.

### Environment Variables Export

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

### Full usage

```
$ ssubmit --help
ssubmit 1.0.0
Michael Hall <michael@mbh.sh>
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

Submit a job with no environment variable export.

$ ssubmit --export NONE -m 2G analysis "python script.py"

Usage: ssubmit [OPTIONS] <NAME> [COMMAND] [-- <REMAINDER>...]

Arguments:
  <NAME>
          Name of the job

          See `man sbatch | grep -A 2 'job-name='` for more details.

  [COMMAND]
          Command to be executed by the job

          For batch jobs, this is required. For interactive jobs (--interactive), 
          this is optional and defaults to starting a shell session.

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

  -T, --test-only
          Return an estimate of when the job would be scheduled to run given the current queue. No job is actually submitted. [sbatch --test-only]

  -i, --interactive
          Request an interactive job session instead of a batch job

          This will use `salloc` instead of `sbatch` and automatically start an interactive shell.
          The command argument becomes optional and defaults to the user's shell.

      --shell <SHELL>
          Shell to use for interactive sessions

          Only used when --interactive is specified. Defaults to the user's login shell.

          [default: auto-detected]

      --export <EXPORT>
          Control which environment variables are exported to the job

          Passed directly to sbatch as --export=<value>. Use 'NONE' to export no variables, 'ALL' to export all variables, or specify specific variables like 'PATH,HOME'.

          [default: ALL]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


[releases]: https://github.com/mbhall88/ssubmit/releases
