use clap::Parser;

use ssubmit::Memory;

/// Submit sbatch jobs without having to create a submission script
///
/// -----------
/// # EXAMPLES
/// -----------
///
/// Submit a simple rsync command
///
/// $ ssubmit rsync_my_data -- rsync -az src/ dest/
///
/// Submit a command that involves piping the output into another command. Note the
/// pipe (`|`) is escaped. This also holds for any other special shell characters.
///
/// $ ssubmit align -- minimap2 ref.fa reads.fq \| samtools sort -o sorted.bam
#[derive(Parser, Debug)]
#[clap(author, version, about, verbatim_doc_comment)]
pub struct Cli {
    /// Name of the job
    ///
    /// See `man sbatch | grep -A 2 'job-name='` for more details.
    pub name: String,
    /// Command to be executed by the job
    #[clap(raw = true, required = true)]
    pub command: Vec<String>,
    /// File to write job stdout to. (See `man sbatch | grep -A 3 'output='`)
    ///
    /// Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.
    #[clap(short, long, default_value = "%x.out")]
    pub output: String,
    /// File to write job stderr to. (See `man sbatch | grep -A 3 'error='`)
    ///
    /// Run `man sbatch | grep -A 37 '^filename pattern'` to see available patterns.
    #[clap(short, long, default_value = "%x.err")]
    pub error: String,
    /// Specify the real memory required per node. e.g., 4.3kb, 7G, 9000, 4.1MB
    ///
    /// Note, floating point numbers will be rounded up. e.g., 10.1G will request 11G.
    /// This is because sbatch only allows integers. See `man sbatch | grep -A 4 'mem='`
    /// for the full details.
    #[clap(
        short = 'r',
        long = "mem",
        value_name = "size[units]",
        default_value = "1G"
    )]
    pub memory: Memory,
    // todo partition
}
