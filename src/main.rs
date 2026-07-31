use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
#[command(infer_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// build and cache current flake for later use
    #[command(visible_alias = "b")]
    Build {
        /// force use of flake from env var `nd_env`, and fail otherwise
        #[arg(short, long)]
        env: bool,
        /// use provided flake location, and fail otherwise
        #[arg(short, long)]
        flake: Option<PathBuf>,
    },

    /// run a command in the latest dev shell
    #[command(visible_alias = "r")]
    Run {
        /// force use of flake from env var `nd_env`, and fail otherwise
        #[arg(short, long)]
        env: bool,
        /// use provided flake location, and fail otherwise
        #[arg(short, long)]
        flake: Option<PathBuf>,
        /// command to run
        #[arg(last = true)]
        command: Vec<String>,
        /// first build if there is no ready environment yet
        #[arg(short, long)]
        build_if_missing: bool,
        /// check and warn if the ready environment is potentially out-of-date
        #[arg(short, long)]
        check: bool,
    },

    /// run an interactive dev shell
    #[command(visible_alias = "s")]
    Shell {
        /// force use of flake from env var `nd_env`, and fail otherwise
        #[arg(short, long)]
        env: bool,
        /// use provided flake location, and fail otherwise
        #[arg(short, long)]
        flake: Option<PathBuf>,
    },

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

fn resolve_flake(env: bool, folder: Option<&Path>) -> Option<PathBuf> {
    // TODO can clap do that for us?
    assert!(
        folder.is_none() || !env,
        "Cannot not use --env and --flake at the same time."
    );

    if let Some(at) = folder {
        if at.to_str().expect("Flake path should be utf8.") == "-" {
            return None;
        } else {
            if !at.join("flake.nix").is_file() {
                panic!("There should be a flake.nix at {}.", at.display());
            }
            return Some(at.into());
        }
    }

    if let Ok(at) = std::env::var("nd_env") {
        let at: PathBuf = at.into();
        if !at.join("flake.nix").is_file() {
            panic!("There should be a flake.nix at {}.", at.display());
        }
        return Some(at);
    }

    if env {
        panic!("The env var `nd_env` should be set.");
    }

    for dir in std::env::current_dir().unwrap().ancestors() {
        let at = dir.join("flake.nix");
        if at.is_file() {
            return Some(at);
        }
    }

    panic!("Cannot find any flake around here.");
}

fn build(folder: &Path) {
    let nd = folder.join(".nd");
    let profile = folder.join(".nd/dev");
    let run = folder.join(".nd/run");

    fs::create_dir_all(nd).expect("Should be able to create the `.nd` folder.");

    let output = Command::new("nix")
        .arg("print-dev-env")
        .arg("--quiet")
        .arg("--quiet")
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

fn check(folder: &Path) {
    let profile = folder.join(".nd/dev");

    let Ok(metadata) = profile.metadata() else {
        println!("Cannot determine how recent the last build is.");
        return;
    };

    let Ok(mtime) = metadata.modified() else {
        println!("Cannot determine how recent the last build is.");
        return;
    };

    let Ok(dt) = SystemTime::now().duration_since(mtime) else {
        println!("Cannot determine how recent the last build is.");
        return;
    };

    if dt > Duration::from_hours(7 * 24) {
        println!("The last build is more than 7 days old.");
    }
}

fn run(folder: Option<&Path>, command: &Vec<String>) {
    if let Some(folder) = folder {
        let run = folder.join(".nd/run");
        // TODO what happens with rusts cleanup if we exec?
        command
            .first()
            .expect("The command needs at least an executable.");
        let e = Command::new(run).args(command).exec();
        panic!("Should be able to exec: {}", e);
    } else {
        // TODO what happens with rusts cleanup if we exec?
        let e = Command::new(
            command
                .first()
                .expect("The command needs at least an executable."),
        )
        .args(&command[1..])
        .exec();
        panic!("Should be able to exec: {}", e);
    };
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Build { env, flake } => {
            if let Some(at) = resolve_flake(env, flake.as_deref()) {
                build(&at);
            }
        }
        Commands::Run {
            env,
            flake,
            command,
            build_if_missing,
            check,
        } => {
            if let Some(at) = resolve_flake(env, flake.as_deref()) {
                if build_if_missing && !at.join(".nd/run").is_file() {
                    build(&at);
                }
                if check {
                    self::check(&at);
                }
                run(Some(&at), &command);
            } else {
                run(None, &command);
            }
        }
        Commands::Shell { env, flake } => {
            let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
            let command = vec![shell];
            if let Some(at) = resolve_flake(env, flake.as_deref()) {
                run(Some(&at), &command);
            } else {
                run(None, &command);
            }
        }
        Commands::Info { env, flake } => {
            if let Some(at) = resolve_flake(env, flake.as_deref()) {
                let at = at.to_str().expect("The flake path should be utf8.");
                println!("Resolving to flake at: {}", at);
            } else {
                println!("Resolving to no flake as requested: Pass through mode.");
            }
        }
    };
}
