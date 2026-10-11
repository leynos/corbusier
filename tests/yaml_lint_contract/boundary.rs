//! The recipes run against the repository with stand-in tools.

use std::{
    error::Error,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Dir, PermissionsExt},
};

static SEQUENCE: AtomicUsize = AtomicUsize::new(0);

/// A stand-in tool in the system temporary directory and its log.
pub struct FakeTool {
    directory: Dir,
    pub tool: Utf8PathBuf,
    log_name: String,
}

impl FakeTool {
    /// Writes an executable stand-in that logs its arguments and exits
    /// with `exit_code`, under names unique to this process and call.
    pub fn new(exit_code: i32) -> Result<Self, Box<dyn Error>> {
        let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let stem = format!("corbusier-yaml-lint-{}-{sequence}", std::process::id());
        let temporary = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| format!("the temporary directory {} is not UTF-8", path.display()))?;
        let directory = Dir::open_ambient_dir(&temporary, ambient_authority())?;
        let log_name = format!("{stem}.log");
        let log = temporary.join(&log_name);
        directory.write(
            &stem,
            format!(
                "#!/bin/sh\nfor argument in \"$@\"; do printf '%s\\n' \"$argument\"; done > '{log}'\nexit {exit_code}\n",
            ),
        )?;
        let mut permissions = directory.metadata(&stem)?.permissions();
        permissions.set_mode(0o755);
        directory.set_permissions(&stem, permissions)?;
        Ok(Self {
            directory,
            tool: temporary.join(stem),
            log_name,
        })
    }

    /// Reads the arguments the tool logged.
    pub fn logged(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.directory.read_to_string(&self.log_name)?)
    }
}

pub fn make(target: &str, variable: &str, tool: &Utf8Path) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new("make")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg(target)
        .arg(format!("{variable}={tool}"))
        .output()?)
}

pub fn tracked_yaml() -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["ls-files", "-z", "*.yml", "*.yaml"])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let mut files: Vec<String> = String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    files.sort_unstable();
    Ok(files)
}

#[test]
fn make_yamllint_hands_the_tool_strict_mode_and_the_tracked_yaml() {
    let fake = FakeTool::new(0).expect("write a stand-in yamllint");

    let output = make("yamllint", "YAMLLINT", &fake.tool).expect("run make yamllint");

    assert!(output.status.success(), "{output:?}");
    let logged = fake.logged().expect("read the stand-in's log");
    let mut arguments = logged.lines();
    assert_eq!(arguments.next(), Some("--strict"));
    assert_eq!(arguments.next(), Some("--"));
    let mut given: Vec<&str> = arguments.collect();
    given.sort_unstable();
    assert_eq!(given, tracked_yaml().expect("list tracked YAML"));
}

#[test]
fn a_failing_yamllint_fails_the_make_target() {
    let fake = FakeTool::new(1).expect("write a stand-in yamllint");

    let output = make("yamllint", "YAMLLINT", &fake.tool).expect("run make yamllint");

    assert!(!output.status.success());
}

#[test]
fn a_failing_actionlint_fails_the_make_target() {
    let fake = FakeTool::new(1).expect("write a stand-in actionlint");

    let output = make("actionlint", "ACTIONLINT", &fake.tool).expect("run make actionlint");

    assert!(!output.status.success());
}

#[test]
fn a_dry_run_of_lint_reaches_both_tools() {
    let output = Command::new("make")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["-n", "lint"])
        .output()
        .expect("run make -n lint");

    let plan = String::from_utf8_lossy(&output.stdout);
    assert!(plan.contains("yamllint --strict"), "{plan}");
    assert!(
        plan.lines().any(|line| line.trim() == "actionlint"),
        "{plan}"
    );
}
