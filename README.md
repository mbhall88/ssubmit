# ssubmit

[![Rust CI](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml/badge.svg)](https://github.com/mbhall88/ssubmit/actions/workflows/ci.yaml)
[![codecov](https://codecov.io/gh/mbhall88/ssubmit/branch/main/graph/badge.svg?token=4O7HTGKD6Q)](https://codecov.io/gh/mbhall88/ssubmit)
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

```shell
$ cargo install ssubmit
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

```
$ ssubmit -h
Submit sbatch jobs without having to create a submission script

Usage: ssubmit [OPTIONS] <NAME> <COMMAND> [-- <REMAINDER>...]

Arguments:
  <NAME>          Name of the job
  <COMMAND>       Command to be executed by the job
  [REMAINDER]...  Options to be passed on to sbatch

Options:
  -o, --output <OUTPUT>    File to write job stdout to. (See `man sbatch | grep -A 3 'output='`) [default: %x.out]
  -e, --error <ERROR>      File to write job stderr to. (See `man sbatch | grep -A 3 'error='`) [default: %x.err]
  -m, --mem <size[unit]>   Specify the real memory required per node. e.g., 4.3kb, 7 Gb, 9000, 4.1MB become 5KB, 7000M, 9000M, and 5M, respectively [env: SSUBMIT_MEMORY=] [default: 1G]
  -t, --time <TIME>        Time limit for the job. e.g. 5d, 10h, 45m21s (case-insensitive) [env: SSUBMIT_TIME=] [default: 1d]
  -S, --shebang <SHEBANG>  The shell shebang for the submission script [env: SSUBMIT_SHEBANG=] [default: "#!/usr/bin/env bash"]
  -s, --set <SET>          Options for the set command in the shell script [env: SSUBMIT_SET=] [default: "euxo pipefail"]
  -n, --dry-run            Print the sbatch command and submission script that would be executed, but do not execute them
  -T, --test-only          Return an estimate of when the job would be scheduled to run given the current queue. No job is actually submitted. [sbatch --test-only]
  -h, --help               Print help (see more with '--help')
  -V, --version            Print version
```

The basic anatomy of a `ssubmit` call is

```
ssubmit [OPTIONS] <NAME> <COMMAND> [-- <REMAINDER>...]
```

`NAME` is the name of the job (the `--job-name` parameter in `sbatch`).

`COMMAND` is what you want to be executed by the job. It **must** be quoted (single or
double).

`REMAINDER` is any (optional) [`sbatch`-specific options](https://slurm.schedmd.com/sbatch.html#lbAG) you want to pass on. These
must follow a `--` after `COMMAND`.

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

### Dry run

You can see what `ssubmit` would do without actually submitting a job using dry run
(`-n,--dry-run`). This will print the `sbatch` command and also the submission script
that would have been provided.

```shell
$ ssubmit -n -m 4g -t 1d dry "rsync -az src/ dest/" -- -c 8
[2022-01-19T08:58:58Z INFO  ssubmit] Dry run requested. Nothing submitted
sbatch -c 8 <script>
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
ssubmit 0.2.0
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
Submit sbatch jobs without having to create a submission script

-----------
# EXAMPLES
-----------

Submit a simple rsync command with a 600MB memory limit.

$ ssubmit -m 600m rsync_my_data "rsync -az src/ dest/"

Submit a command that involves piping the output into another command. sbatch options
are passed after a `--`.

$ ssubmit -m 4G align "minimap2 -t 8 ref.fa reads.fq | samtools sort -o sorted.bam" -- -c 8

Usage: ssubmit [OPTIONS] <NAME> <COMMAND> [-- <REMAINDER>...]

Arguments:
  <NAME>
          Name of the job

          See `man sbatch | grep -A 2 'job-name='` for more details.

  <COMMAND>
          Command to be executed by the job

  [REMAINDER]...
          Options to be passed on to sbatch

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

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


[releases]: https://github.com/mbhall88/ssubmit/releases
