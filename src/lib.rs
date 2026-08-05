use std::{
    ffi::OsString,
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Volume {
    /// no output at all
    Silent,
    /// very little output, "nice to watch"
    Quiet,
    /// normal output
    Normal,
}

impl Volume {
    fn includes(&self, vol: &Volume) -> bool {
        self >= vol // meaning the setting is as loud or louder than the volume in question
    }
}

static VOLUME: OnceLock<Volume> = OnceLock::new();

pub fn set_volume(quiet: bool, silent: bool) {
    let v = if silent {
        Volume::Silent
    } else if quiet {
        Volume::Quiet
    } else {
        Volume::Normal
    };
    VOLUME.set(v).unwrap();
}

pub fn is_volume_included(vol: &Volume) -> bool {
    VOLUME.get().unwrap_or(&Volume::Normal).includes(vol)
}

#[allow(unused_macros)]
macro_rules! uprintln {
    ($vol:expr, $($arg:tt)*) => {
        if is_volume_included(&$vol) {
            println!($($arg)*);
        }
    };
}

#[allow(unused_macros)]
macro_rules! ueprintln {
    ($vol:expr, $($arg:tt)*) => {
        if is_volume_included(&$vol) {
            eprintln!($($arg)*);
        }
    };
}

pub enum Flake {
    Default,
    Here,
    Env,
    At { at: PathBuf },
}

fn resolve_flake(flake: &Flake) -> Option<PathBuf> {
    let path = match flake {
        Flake::Default => resolve_flake_default(),
        Flake::Here => resolve_flake_here(),
        Flake::Env => resolve_flake_env(),
        Flake::At { at } => resolve_flake_at(at),
    };
    match path {
        Some(ref path) => uprintln!(Volume::Quiet, "Using flake at {}.", path.display()),
        None => uprintln!(Volume::Quiet, "Using no flake, pass-through mode."),
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
            return Some(dir.into());
        }
    }
    panic!("Cannot find any flake around here.");
}

fn resolve_flake_env() -> Option<PathBuf> {
    let at = std::env::var("nd_env").expect("The env var `nd_env` should be set.");
    let at: PathBuf = at.into();
    resolve_flake_at(&at)
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

pub fn build(flake: &Flake, if_missing: bool) {
    let Some(folder) = resolve_flake(flake) else {
        return;
    };

    let nd = folder.join(".nd");
    let profile = folder.join(".nd/dev");
    let run = folder.join(".nd/run");

    if if_missing && run.is_file() {
        return;
    }

    fs::create_dir_all(nd).expect("Should be able to create the `.nd` folder.");

    uprintln!(Volume::Normal, "Building flake at {}.", folder.display());

    let mut cmd = Command::new("nix");

    cmd.arg("print-dev-env")
        .arg("--profile")
        .arg(&profile)
        .arg(&folder);

    if is_volume_included(&Volume::Normal) {
        cmd.stderr(Stdio::inherit());
    }

    let output = cmd.output().expect("Should be able to run `nix`");

    if !output.status.success() {
        panic!(
            "Running `nix print-dev-env` was not successful:\n{}",
            String::from_utf8_lossy(&output.stderr),
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
        "#!/usr/bin/env bash\n\
         \n\
         original_shell=${{SHELL:-sh}}\n\
         \n\
         {dev_env}\n\
         \n\
         export SHELL=$original_shell\n\
         export nd_nix={profile_str}\n\
         if [[ -v nd ]]; then export nd={folder_str}:$nd; else export nd={folder_str}; fi\n\
         \n\
         exec \"$@\"\n\
        "
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

pub fn maybe_warn(flake: &Flake) {
    let Some(folder) = resolve_flake(flake) else {
        return;
    };

    match is_latest_build_old(&folder) {
        Some(false) => {}
        Some(true) => uprintln!(Volume::Normal, "The last build is more than 7 days old."),
        None => uprintln!(
            Volume::Normal,
            "Cannot determine how recent the last build is."
        ),
    }

    match is_latest_lock_different(&folder) {
        Some(false) => {}
        Some(true) => uprintln!(
            Volume::Normal,
            "Flake.lock has changed since the last build."
        ),
        None => uprintln!(
            Volume::Normal,
            "Cannot determine if flake.lock has changed."
        ),
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

pub fn run(flake: &Flake, command: &[String], build_if_missing: bool, warn: bool, build: bool) {
    assert!(
        !command.is_empty(),
        "The command needs at least an executable."
    );

    let (run, args): (OsString, &[String]) = if let Some(folder) = resolve_flake(flake) {
        if build || (build_if_missing && !folder.join(".nd/run").is_file()) {
            self::build(flake, build_if_missing);
        }
        if warn {
            maybe_warn(flake);
        }
        (folder.join(".nd/run").into(), command)
    } else {
        let (run, args) = command.split_first().unwrap();
        (run.into(), args)
    };

    let e = Command::new(run).args(args).exec();
    panic!("Should be able to exec: {}", e);
}

pub fn run_shell(flake: &Flake, args: &[String], build_if_missing: bool, warn: bool, build: bool) {
    let shell = std::env::var("SHELL").unwrap_or(String::from("sh"));
    let mut command = vec![shell];
    command.extend_from_slice(args);
    run(flake, &command, build_if_missing, warn, build);
}

pub fn info(flake: &Flake) {
    uprintln!(
        Volume::Normal,
        "$nd_env: {}",
        std::env::var("nd_env").unwrap_or("<not set>".into())
    );
    uprintln!(
        Volume::Normal,
        "$nd: {}",
        std::env::var("nd").unwrap_or("<not set>".into())
    );
    uprintln!(
        Volume::Normal,
        "$nd_nix: {}",
        std::env::var("nd_nix").unwrap_or("<not set>".into())
    );
    if let Some(at) = resolve_flake(flake) {
        uprintln!(Volume::Normal, "Resolving to flake at: {}", at.display());
        maybe_warn(flake);
    } else {
        uprintln!(
            Volume::Normal,
            "Resolving to no flake as requested: Pass through mode."
        );
    }
}
