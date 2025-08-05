use std::cmp::max;
use std::fmt::Write as _;
use std::time::Duration; // import without risk of name clashing

use log::warn;

static SCRIPT_TEMPLATE: &str = r#"$shebang$
#SBATCH --job-name=$name$
#SBATCH --mem=$memory$
#SBATCH --time=$time$
#SBATCH --error=$error$
#SBATCH --output=$output$
$set$

$cmd$
"#;

#[allow(clippy::too_many_arguments)]
pub fn make_submission_script(
    shebang: &str,
    set: &str,
    name: &str,
    memory: &str,
    time: &str,
    error: &str,
    output: &str,
    cmd: &str,
) -> String {
    let mut set_line = String::new();
    if !set.is_empty() {
        let _ = write!(set_line, "set -{set}");
    }
    let script = SCRIPT_TEMPLATE
        .replace("$shebang$", shebang)
        .replace("$name$", name)
        .replace("$memory$", memory)
        .replace("$time$", time)
        .replace("$error$", error)
        .replace("$output$", output)
        .replace("$cmd$", cmd)
        .replace("$set$", &set_line);

    if memory == "0" {
        warn!("Memory provided was 0; using cluster default. Use `scontrol show config | grep -i 'DefMem'` to check the default memory.");
        script
            .split_inclusive('\n')
            .filter(|line| !line.contains("--mem"))
            .collect::<Vec<&str>>()
            .concat()
    } else {
        script
    }
}

pub trait SlurmTime {
    fn to_slurm_time(&self) -> String;
}

impl SlurmTime for Duration {
    fn to_slurm_time(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }

        let mut remainder = max(self.as_secs(), 1);

        if remainder < 60 {
            // less than a minute
            return format!("0:{remainder}");
        }

        let secs = remainder % 60;
        remainder /= 60;

        if remainder < 60 {
            // less than an hour
            return format!("{remainder}:{secs}");
        }

        let mins = remainder % 60;
        remainder /= 60;

        format!("{remainder}:{mins}:{secs}")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_to_slurm_zero() {
        let one_milli = Duration::from_millis(0);

        let actual = one_milli.to_slurm_time();
        let expected = "0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_second() {
        let one_milli = Duration::from_millis(6);

        let actual = one_milli.to_slurm_time();
        let expected = "0:1";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_minute() {
        let secs = Duration::from_secs(6);

        let actual = secs.to_slurm_time();
        let expected = "0:6";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_less_than_one_hour() {
        let secs = Duration::from_secs(64);

        let actual = secs.to_slurm_time();
        let expected = "1:4";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_minutes_with_remainder() {
        let secs = Duration::from_secs(666);

        let actual = secs.to_slurm_time();
        let expected = "11:6";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_even_minutes() {
        let secs = Duration::from_secs(60);

        let actual = secs.to_slurm_time();
        let expected = "1:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_even_hours() {
        let secs = Duration::from_secs(60 * 60 * 4);

        let actual = secs.to_slurm_time();
        let expected = "4:0:0";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_hours_with_remainder() {
        let secs = Duration::from_secs(9042);

        let actual = secs.to_slurm_time();
        let expected = "2:30:42";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_to_slurm_over_a_day() {
        let secs = Duration::from_secs(561677);

        let actual = secs.to_slurm_time();
        let expected = "156:1:17";

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_make_submission_script() {
        let shebang = "#/bin/bash";
        let set = "eux";
        let name = "job";
        let memory = "1M";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --mem={memory}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}
set -{set}

{cmd}
"#
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_make_submission_script_no_set() {
        let shebang = "#/bin/bash";
        let set = "";
        let name = "job";
        let memory = "1M";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --mem={memory}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}


{cmd}
"#
        );
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_make_submission_script_mem_is_zero() {
        let shebang = "#/bin/bash";
        let set = "";
        let name = "job";
        let memory = "0";
        let time = "5:56:00";
        let error = "%x.err";
        let output = "%x.out";
        let cmd = "python -c 'print(1+1)'";

        let actual = make_submission_script(shebang, set, name, memory, time, error, output, cmd);
        let expected = format!(
            r#"{shebang}
#SBATCH --job-name={name}
#SBATCH --time={time}
#SBATCH --error={error}
#SBATCH --output={output}


{cmd}
"#
        );
        assert_eq!(actual, expected)
    }
}
