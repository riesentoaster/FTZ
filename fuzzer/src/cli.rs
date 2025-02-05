use std::path::PathBuf;

use clap::{self, Parser};

use libafl_bolts::core_affinity::Cores;

/// The commandline args this fuzzer accepts
#[derive(Debug, Parser)]
#[command(
    name = "zephyr_net_fuzzer",
    about = "A fuzzer for the TCP/IP stack of Zephyr",
    author = "Valentin Huber <contact@valentinhuber.me>"
)]
pub struct Cli {
    #[arg(
    short,
    long,
    value_parser = Cores::from_cmdline,
    help = "Spawn a client in each of the provided cores. Broker runs in the 0th core. 'all' to select all available cores. 'none' to run a client without binding to any core. eg: '1,2-4,6' selects the cores 1,2,3,4,6.",
    name = "CORES",
    default_value = "0"
    )]
    cores: Cores,

    #[arg(
        short,
        long,
        help = "Spawn n clients on each of the provided cores.",
        name = "OVERCOMMIT",
        default_value = "1"
    )]
    overcommit: usize,

    #[arg(
        short,
        long,
        action,
        help = "Only run a single iteration of the fuzzer. Overrides cores and overcommmit to 1 each.",
        name = "FUZZ_ONE"
    )]
    fuzz_one: bool,

    #[arg(
        short,
        long,
        action,
        help = "Only load/generate the corpus. Do not perform any fuzzing.",
        name = "LOAD_ONLY"
    )]
    load_only: bool,

    #[arg(
        short,
        long,
        help = "Set the Zephyr executable path",
        name = "ZEPHYR_EXEC_PATH",
        required = true
    )]
    zephyr_exec_dir: PathBuf,

    #[arg(
        short,
        long,
        help = "Set the Zephyr real-time ratio, 2 means zephyr runs twice as fast as real-time",
        name = "ZEPHYR_RT_RATIO",
        default_value = "1"
    )]
    zephyr_rt_ratio: f64,

    #[arg(
        short,
        long,
        help = "Redirect Zephyr's output to this file",
        name = "ZEPHYR_OUT_DIR"
    )]
    zephyr_out_dir: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Set the corpus directory",
        name = "CORPUS_DIR",
        default_value = "corpus"
    )]
    corpus_dir: PathBuf,

    #[arg(
        short,
        long,
        help = "Set the solutions directory",
        name = "SOLUTIONS_DIR",
        default_value = "solutions"
    )]
    solutions_dir: PathBuf,

    #[arg(short, long, help = "Set the stdout path", name = "STDOUT")]
    stdout: Option<PathBuf>,

    #[arg(short, long, help = "Set the stderr path", name = "STDERR")]
    stderr: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Set the monitor path",
        name = "MONITOR",
        default_value = "monitor"
    )]
    monitor: String,

    #[arg(
        short,
        long,
        action,
        help = "Calculate state transitions.",
        name = "STATE_DIFF"
    )]
    state_diff: bool,
}

impl Cli {
    pub fn cores(&self) -> &Cores {
        &self.cores
    }

    pub fn zephyr_exec_dir(&self) -> &PathBuf {
        &self.zephyr_exec_dir
    }

    pub fn stdout(&self) -> Option<&PathBuf> {
        self.stdout.as_ref()
    }

    pub fn stderr(&self) -> Option<&PathBuf> {
        self.stderr.as_ref()
    }

    pub fn overcommit(&self) -> usize {
        self.overcommit
    }

    pub fn fuzz_one(&self) -> bool {
        self.fuzz_one
    }

    pub fn load_only(&self) -> bool {
        self.load_only
    }

    pub fn zephyr_rt_ratio(&self) -> f64 {
        self.zephyr_rt_ratio
    }

    pub fn zephyr_out_dir(&self) -> Option<&PathBuf> {
        self.zephyr_out_dir.as_ref()
    }

    pub fn monitor(&self) -> &str {
        &self.monitor
    }

    pub fn state_diff(&self) -> bool {
        self.state_diff
    }

    pub fn corpus_dir(&self) -> &PathBuf {
        &self.corpus_dir
    }

    pub fn solutions_dir(&self) -> &PathBuf {
        &self.solutions_dir
    }
}
