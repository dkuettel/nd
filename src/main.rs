use std::{
    ffi::OsString,
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
#[command(infer_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// build and save a dev shell in a profile for later use
    // TODO make b an alias for `build this` the normal thing? same for run and co?
    #[command(visible_alias = "b")]
    Build {
        /// which flake to use
        #[command(flatten)]
        flake: Flake,
        /// no output, maybe
        #[arg(short, long)]
        quiet: bool,
    },

    /// run a command in the latest dev shell
    #[command(visible_alias = "r")]
    Run {
        /// which flake to use
        #[command(flatten)]
        flake: Flake,
        /// command to run
        #[arg(last = true)]
        command: Vec<String>,
        /// opts
        #[command(flatten)]
        opts: RunOpts,
    },

    /// run an interactive dev shell
    #[command(visible_alias = "s")]
    Shell {
        /// which flake to use
        #[command(flatten)]
        flake: Flake,
        /// opts
        #[command(flatten)]
        opts: RunOpts,
    },

    // TODO should also say things like which env vars are set, and if we are in an env right now
    /// show general info
    #[command(visible_alias = "i")]
    Info {
        /// force use of flake from env var `nd_env`, and fail otherwise
        #[arg(short, long)]
        env: bool,
        /// use provided flake location, and fail otherwise
        #[arg(short, long)]
        flake: Option<PathBuf>,
    },
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
struct Flake {
    /// find a flake here or in parent directories, don't use env var `nd_env`
    #[arg(short = 'H', long)]
    here: bool,
    /// force use of flake from env var `nd_env`, and fail otherwise
    #[arg(short, long)]
    env: bool,
    /// use provided flake location, and fail otherwise
    #[arg(short, long)]
    at: Option<PathBuf>,
}

impl Flake {
    fn as_spec(self) -> FlakeSpec {
        if let Some(at) = self.at {
            return FlakeSpec::At { at };
        }
        if self.env {
            return FlakeSpec::Env;
        }
        if self.here {
            return FlakeSpec::Here;
        }
        FlakeSpec::Default
    }
}

enum FlakeSpec {
    Default,
    Here,
    Env,
    At { at: PathBuf },
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

// TODO should we support pass-thru? i think we need it for tmux
fn resolve_flake(spec: &FlakeSpec, quiet: bool) -> Option<PathBuf> {
    let path = match spec {
        FlakeSpec::Default {} => resolve_flake_default(),
        FlakeSpec::Here {} => resolve_flake_here(),
        FlakeSpec::Env {} => resolve_flake_env(),
        FlakeSpec::At { at } => resolve_flake_at(at),
    };
    if !quiet {
        match path {
            Some(ref path) => println!("Using flake at {}.", path.display()),
            None => println!("Using no flake, pass-through mode."),
        }
    }
    path
}

fn resolve_flake_default() -> Option<PathBuf> {
    if std::env::var("nd_env").is_ok() {
        resolve_flake_env()
    } else {
        resolve_flake_here()
    }
}

fn resolve_flake_here() -> Option<PathBuf> {
    for dir in std::env::current_dir().unwrap().ancestors() {
        let at = dir.join("flake.nix");
        if at.is_file() {
            return Some(at);
        }
    }
    panic!("Cannot find any flake around here.");
}

fn resolve_flake_env() -> Option<PathBuf> {
    let at = std::env::var("nd_env").expect("The env var `nd_env` should be set.");
    let at: PathBuf = at.into();
    if !at.join("flake.nix").is_file() {
        panic!("There should be a flake.nix at {}.", at.display());
    }
    Some(at)
}

fn resolve_flake_at(at: &Path) -> Option<PathBuf> {
    if at.to_str().expect("Flake path should be utf8.") == "-" {
        None
    } else {
        if !at.join("flake.nix").is_file() {
            panic!("There should be a flake.nix at {}.", at.display());
        }
        Some(at.into())
    }
}

fn build(folder: &Path, quiet: bool) {
    let nd = folder.join(".nd");
    let profile = folder.join(".nd/dev");
    let run = folder.join(".nd/run");

    fs::create_dir_all(nd).expect("Should be able to create the `.nd` folder.");

    if !quiet {
        println!("Building flake at {}.", folder.display());
    }

    let quiet_args = if quiet {
        vec!["--quiet", "--quiet"]
    } else {
        vec![]
    };

    let output = Command::new("nix")
        .arg("print-dev-env")
        .args(quiet_args)
        .arg("--profile")
        .arg(&profile)
        .arg(folder)
        .output()
        .expect("Should run `nix print-dev-env` successfully.");

    let profile = profile
        .canonicalize()
        .expect("Should be able to follow `.nd/dev` symlinks to the nix store.");
    let profile_str = profile.to_str().unwrap();
    let folder_str = folder.to_str().unwrap();

    let mut script = String::new();
    script.push_str("#!/usr/bin/env bash\n");
    script.push('\n');
    script.push_str(
        str::from_utf8(&output.stdout).expect("`nix print-dev-env` should produce utf8 output."),
    );
    script.push_str("\n\n");
    script.push_str(format!("export nd_nix={}\n", profile_str).as_str());
    script.push_str(
        format!(
            "if [[ -v nd ]]; then export nd={0}:$nd; else export nd={0}; fi\n",
            folder_str
        )
        .as_str(),
    );
    script.push('\n');
    script.push_str("exec \"$@\"\n");

    fs::write(&run, script.as_str())
        .expect("Should have write access for the `.nd/run` script file.");

    fs::set_permissions(&run, fs::Permissions::from_mode(0o755))
        .expect("Should have write access for the `.nd/run` script file.");

    let lock = folder.join("flake.lock");
    let nd_lock = folder.join(".nd/flake.lock");
    fs::copy(lock, nd_lock).expect("There should be a flake.lock.");
}

fn is_latest_build_old(folder: &Path) -> Option<bool> {
    let profile = folder.join(".nd/dev");
    let metadata = profile.metadata().ok()?;
    let mtime = metadata.modified().ok()?;
    let dt = SystemTime::now().duration_since(mtime).ok()?;
    Some(dt > Duration::from_hours(7 * 24))
}

fn is_latest_lock_different(folder: &Path) -> Option<bool> {
    let current = folder.join("flake.lock");
    let current_data = fs::read(current).ok()?;

    let latest = folder.join(".nd/flake.lock");
    let latest_data = fs::read(latest).ok()?;

    Some(current_data != latest_data)
}

fn maybe_warn(folder: &Path) {
    match is_latest_build_old(folder) {
        Some(false) => {}
        Some(true) => println!("The last build is more than 7 days old."),
        None => println!("Cannot determine how recent the last build is."),
    }
    match is_latest_lock_different(folder) {
        Some(false) => {}
        Some(true) => println!("Flake.lock has changed since the last build."),
        None => println!("Cannot determine if flake.lock has changed."),
    }
}

fn run(folder: Option<&Path>, command: &[String]) {
    assert!(
        !command.is_empty(),
        "The command needs at least an executable."
    );

    let (run, args): (OsString, &[String]) = if let Some(folder) = folder {
        (folder.join(".nd/run").into(), command)
    } else {
        let (run, args) = command.split_first().unwrap();
        (run.into(), args)
    };

    // TODO what happens with rusts cleanup if we exec?
    let e = Command::new(run).args(args).exec();
    panic!("Should be able to exec: {}", e);
}

fn cli_run(flake: Flake, command: &[String], opts: &RunOpts) {
    let spec = flake.as_spec();
    if let Some(at) = resolve_flake(&spec, false) {
        if opts.build || (opts.build_if_missing && !at.join(".nd/run").is_file()) {
            self::build(&at, false);
        }
        if opts.warn {
            maybe_warn(&at);
        }
        run(Some(&at), command);
    } else {
        run(None, command);
    }
}

fn main() {
    let args = Cli::parse();

    println!("{:?}", args);

    match args.command {
        Commands::Build { flake, quiet } => {
            let spec = flake.as_spec();
            if let Some(at) = resolve_flake(&spec, quiet) {
                build(&at, quiet);
            }
        }
        Commands::Run {
            flake,
            command,
            opts,
        } => {
            cli_run(flake, &command, &opts);
        }
        Commands::Shell { flake, opts } => {
            let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
            let command = vec![shell];
            cli_run(flake, &command, &opts);
        }
        Commands::Info { env, flake } => {
            if let Some(at) = resolve_flake(&FlakeSpec::Default, false) {
                let at = at.to_str().expect("The flake path should be utf8.");
                println!("Resolving to flake at: {}", at);
            } else {
                println!("Resolving to no flake as requested: Pass through mode.");
            }
        }
    };
}
