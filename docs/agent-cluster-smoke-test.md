# Agent workflow smoke test

Run this checklist manually on a Slurm login or submission node before release.
It is deliberately not part of CI, which must remain usable without Slurm.
Record the node, `ssubmit --version`, Slurm version and date with the result.

Replace `<known-good-partition>` with a partition supplied by the cluster user.
The invalid-partition test uses a deliberately unlikely name; if that name is
defined at the site, replace it with another name that the site confirms does
not exist. Do not add a guessed account, QoS or partition to a normal workflow.

1. Confirm local executables:

   ```sh
   command -v ssubmit
   command -v sbatch
   ```

2. Plan without submitting. Confirm that `sbatch` is not invoked and that stdout
   is one JSON object with `operation` set to `plan`:

   ```sh
   ssubmit --dry-run --json smoke-plan 'printf "planned\\n"' \
     --mem 128M --time 2m -- --cpus-per-task=1
   ```

3. Ask Slurm for a schedule estimate. Confirm `operation: "test"`, scheduler
   feedback in `test.stdout` or `test.stderr`, and no submitted job:

   ```sh
   ssubmit --test-only --json smoke-test 'printf "tested\\n"' \
     --mem 128M --time 2m -- --partition=<known-good-partition>
   ```

4. Submit a trivial successful batch job. Confirm a zero exit status, a string
   `submission.job_id`, an optional `submission.cluster`, and the output/error
   patterns in `plan.job`:

   ```sh
   ssubmit --json smoke-success 'printf "success\\n"' \
     --mem 128M --time 2m -- --partition=<known-good-partition>
   ```

   If `jq` is available, inspect the identity without scraping logs:

   ```sh
   ssubmit --json smoke-parse 'printf "parse\\n"' \
     --mem 128M --time 2m -- --partition=<known-good-partition> |
     jq '{job_id: .submission.job_id, cluster: .submission.cluster,
          output: .plan.job.output, error: .plan.job.error}'
   ```

5. Submit with an invalid partition. Confirm a non-zero exit status, one JSON
   error object on stdout, `ok: false`, a Slurm error kind and diagnostics on the
   JSON error or stderr:

   ```sh
   if ssubmit --json smoke-invalid 'true' --mem 128M --time 2m \
     -- --partition=__ssubmit_invalid_partition__; then
     echo 'unexpected success' >&2
     exit 1
   fi
   ```

6. Run the unchanged human interface. Confirm its normal human-readable output
   and exit behaviour remain intact:

   ```sh
   ssubmit --dry-run smoke-human 'printf "human\\n"' \
     --mem 128M --time 2m -- --partition=<known-good-partition>
   ```

Clean up the trivial jobs and generated output files according to the site's
normal Slurm policy. Do not include credentials or cluster-specific values in
the repository.
