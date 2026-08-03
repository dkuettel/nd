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
    #[command(visible_alias = "b")]
    Build {
        #[command(flatten)]
        spec: FlakeSpec,
        /// no output, maybe
        #[arg(short, long)]
        quiet: bool,
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

    /// run an interactive dev shell
    #[command(visible_alias = "s")]
    Shell {
        #[command(flatten)]
        spec: FlakeSpec,
        #[command(flatten)]
        opts: RunOpts,
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

impl FlakeSpec {
    fn as_flake(&self) -> Flake {
        // TODO not validating yet if all args make sense, clap doesnt offer it in a typed manner I
        // think
        if let Some(ref at) = self.at {
            return Flake::At { at: at.clone() };
        }
        if self.env {
            return Flake::Env;
        }
        if self.here {
            return Flake::Here;
        }
        Flake::Default
    }
}

enum Flake {
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
fn resolve_flake(flake: &Flake, quiet: bool) -> Option<PathBuf> {
    let path = match flake {
        Flake::Default => resolve_flake_default(),
        Flake::Here => resolve_flake_here(),
        Flake::Env => resolve_flake_env(),
        Flake::At { at } => resolve_flake_at(at),
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
        .expect("Should be able to run `nix`");

    if !output.status.success() {
        panic!(
            "Should run `nix print-dev-env` successfully: {}",
            String::from_utf8(output.stderr).unwrap_or(String::from("Cannot read stderr."))
        );
    }

    let profile = profile
        .canonicalize()
        .expect("Should be able to follow `.nd/dev` symlinks to the nix store.");
    let profile_str = profile.to_str().unwrap();
    let folder_str = folder.to_str().unwrap();

    let dev_env =
        String::from_utf8(output.stdout).expect("`nix print-dev-env` should produce utf8 output.");
    let script = format!(
        "#!/usr/bin/env bash\n\n\
         {dev_env}\n\n\
         export nd_nix={profile_str}\n\
         if [[ -v nd ]]; then export nd={folder_str}:$nd; else export nd={folder_str}; fi\n\n\
         exec \"$@\"\n"
    );

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
    let metadata = profile.symlink_metadata().ok()?;
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

// NOTE this is the format of .nd/dev, and we could extract most of the env vars from this
// but it is still difficult to know which ones we have to set, and which ones we have to append
// and then there is also the shellHook, which requires us to run a script anyway, so maybe we cant
// do much with this
// #[derive(Deserialize, Debug)]
// struct DevShell {
//     #[serde(rename = "bashFunctions")]
//     bash_functions: HashMap<String, String>,
//     variables: HashMap<String, DevShellVariable>,
// }
// #[derive(Deserialize, Debug)]
// #[serde(tag = "type")]
// #[serde(rename_all = "lowercase")]
// enum DevShellVariable {
//     Exported { value: String },
//     Var { value: String },
//     Unknown { value: Option<String> },
//     Array { value: Vec<String> },
// }

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

fn cli_run(spec: &FlakeSpec, command: &[String], opts: &RunOpts) {
    let spec = spec.as_flake();
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

    match args.command {
        Commands::Build { spec, quiet } => {
            let flake = spec.as_flake();
            if let Some(at) = resolve_flake(&flake, quiet) {
                build(&at, quiet);
            }
        }
        Commands::Run {
            spec,
            command,
            opts,
        } => {
            cli_run(&spec, &command, &opts);
        }
        Commands::Shell { spec, opts } => {
            let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
            let command = vec![shell];
            cli_run(&spec, &command, &opts);
        }
        Commands::Info { spec } => {
            println!(
                "$nd_env: {}",
                std::env::var("nd_env").unwrap_or("<not set>".into())
            );
            println!("$nd: {}", std::env::var("nd").unwrap_or("<not set>".into()));
            println!(
                "$nd_nix: {}",
                std::env::var("nd_nix").unwrap_or("<not set>".into())
            );
            let flake = spec.as_flake();
            if let Some(at) = resolve_flake(&flake, true) {
                println!("Resolving to flake at: {}", at.display());
                maybe_warn(&at);
            } else {
                println!("Resolving to no flake as requested: Pass through mode.");
            }
        }
    };
}
