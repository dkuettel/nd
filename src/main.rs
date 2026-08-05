use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
#[command(infer_subcommands = true)]
struct Cli {
    /// little ouptut
    #[arg(short, long)]
    quiet: bool,
    /// no output
    #[arg(short, long)]
    silent: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// build and save a dev shell in a profile for later use
    #[command(visible_alias = "b")]
    Build {
        #[command(flatten)]
        spec: FlakeSpec,
        /// only build if missing
        #[arg(short = 'm', long)]
        if_missing: bool,
    },

    /// run a command in the latest dev shell
    #[command(visible_alias = "r")]
    Run {
        #[command(flatten)]
        spec: FlakeSpec,
        /// command to run
        #[arg(last = true)]
        command: Vec<String>,
        #[command(flatten)]
        opts: RunOpts,
    },

    /// run an interactive dev shell using $SHELL
    #[command(visible_alias = "s")]
    Shell {
        #[command(flatten)]
        spec: FlakeSpec,
        #[command(flatten)]
        opts: RunOpts,
        /// additional args to the $SHELL executable
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// show general info
    #[command(visible_alias = "i")]
    Info {
        #[command(flatten)]
        spec: FlakeSpec,
    },
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
struct FlakeSpec {
    /// (default) first try env var `nd_env`, then try parent directories
    #[arg(short = 'A', long, help_heading = "Flake specification")]
    any: bool,
    /// find a flake here or in parent directories, don't use env var `nd_env`
    #[arg(short = 'H', long, help_heading = "Flake specification")]
    here: bool,
    /// force use of flake from env var `nd_env`, and fail otherwise
    #[arg(short, long, help_heading = "Flake specification")]
    env: bool,
    /// use provided flake location, and fail otherwise
    #[arg(short, long, help_heading = "Flake specification")]
    at: Option<PathBuf>,
}

impl From<FlakeSpec> for nd::Flake {
    fn from(value: FlakeSpec) -> Self {
        // TODO not validating yet if all args make sense, clap doesnt offer it in a typed manner I think
        if let Some(ref at) = value.at {
            return nd::Flake::At { at: at.clone() };
        }
        if value.env {
            return nd::Flake::Env;
        }
        if value.here {
            return nd::Flake::Here;
        }
        nd::Flake::Default
    }
}

#[derive(Args, Debug)]
struct RunOpts {
    /// first build if there is no ready environment yet
    #[arg(short = 'm', long)]
    build_if_missing: bool,
    /// warn if the ready environment is potentially out-of-date
    #[arg(short, long)]
    warn: bool,
    /// build before running (always, no attempt is made to figure out if a build is necessary
    /// other than what nix does itself, which means at least the context is built and copied to
    /// the nix store)
    #[arg(short, long)]
    build: bool,
}

fn main() {
    let args = Cli::parse();

    nd::set_volume(args.quiet, args.silent);

    match args.command {
        Commands::Build { spec, if_missing } => nd::build(&spec.into(), if_missing),
        Commands::Run {
            spec,
            command,
            opts,
        } => nd::run(
            &spec.into(),
            &command,
            opts.build_if_missing,
            opts.warn,
            opts.build,
        ),
        Commands::Shell { spec, opts, args } => nd::run_shell(
            &spec.into(),
            &args,
            opts.build_if_missing,
            opts.warn,
            opts.build,
        ),
        Commands::Info { spec } => nd::info(&spec.into()),
    };
}
