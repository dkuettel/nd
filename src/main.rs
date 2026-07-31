use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
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

fn resolve_flake(
    env: bool,
    folder: Option<&Path>,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    // TODO can clap do that for us?
    assert!(
        folder.is_none() || !env,
        "Cannot not use --env and --flake at the same time."
    );

    if let Some(at) = folder {
        if at.to_str().expect("Flake path should be utf8.") == "-" {
            return Ok(None);
        } else {
            return Ok(Some(at.into()));
        }
    }

    if let Ok(at) = std::env::var("nd_env") {
        return Ok(Some(at.into()));
    }

    if env {
        return Err("The env var `nd_env` should be set.".into());
    }

    for dir in std::env::current_dir()?.ancestors() {
        let at = dir.join("flake.nix");
        if at.is_file() {
            return Ok(Some(dir.into()));
        }
    }

    Err("cannot find any flake".into())
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

    let mut script = String::new();
    script.push_str("#!/usr/bin/env bash\n");
    script.push('\n');
    script.push_str(
        str::from_utf8(&output.stdout).expect("`nix print-dev-env` should produce utf8 output."),
    );
    script.push_str("\n\n");
    script.push_str(format!("export nd_nix={}\n", profile_str).as_str());
    script.push_str("exec \"$@\"\n");

    fs::write(&run, script.as_str())
        .expect("Should have write access for the `.nd/run` script file.");

    fs::set_permissions(&run, fs::Permissions::from_mode(0o755))
        .expect("Should have write access for the `.nd/run` script file.");
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Build { env, flake } => {
            if let Some(at) = resolve_flake(env, flake.as_deref())? {
                build(&at);
            }
        }
        Commands::Run {
            env,
            flake,
            command,
        } => {
            if let Some(at) = resolve_flake(env, flake.as_deref())? {
                run(Some(&at), &command);
            } else {
                run(None, &command);
            }
        }
        Commands::Shell { env, flake } => {
            let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
            let command = vec![shell];
            if let Some(at) = resolve_flake(env, flake.as_deref())? {
                run(Some(&at), &command);
            } else {
                run(None, &command);
            }
        }
        Commands::Info { env, flake } => {
            if let Some(at) = resolve_flake(env, flake.as_deref())? {
                let at = at.to_str().expect("The flake path should be utf8.");
                println!("Resolving to flake at: {}", at);
            } else {
                println!("Resolving to no flake as requested: Pass through mode.");
            }
        }
    };

    Ok(())
}
