//! Behavioural tests for the `pg_worker` binary.

#[cfg(unix)]
mod unix_tests {
    //! Behavioural tests for Unix `pg_worker` execution.

    use cap_std::ambient_authority;
    use cap_std::fs::Dir;
    use eyre::{Result, ensure, eyre};
    use std::ffi::OsString;
    use std::io::Write;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::{Command, Output};

    fn worker_path() -> Result<PathBuf> {
        std::env::var_os("CARGO_BIN_EXE_pg_worker")
            .map(PathBuf::from)
            .ok_or_else(|| {
                eyre!(
                    "CARGO_BIN_EXE_pg_worker is not set; ensure the pg_worker binary is built"
                )
            })
    }

    fn run_worker(args: &[OsString]) -> Result<Output> {
        let path = worker_path()?;
        Command::new(&path)
            .args(args)
            .output()
            .map_err(|err| eyre!(err))
    }

    fn open_temp_dir() -> Result<(Dir, PathBuf)> {
        let path = std::env::temp_dir();
        let dir = Dir::open_ambient_dir(&path, ambient_authority()).map_err(|err| eyre!(err))?;
        Ok((dir, path))
    }

    fn write_temp_config(contents: &str) -> Result<PathBuf> {
        let filename = format!("pg_worker_test_{}.json", uuid::Uuid::new_v4());
        let (dir, path) = open_temp_dir()?;
        let mut file = dir.create(&filename).map_err(|err| eyre!(err))?;
        file.write_all(contents.as_bytes())
            .map_err(|err| eyre!(err))?;
        Ok(path.join(filename))
    }

    fn remove_temp_file(path: &Path) -> Result<()> {
        let (dir, _) = open_temp_dir()?;
        let file_name = path
            .file_name()
            .ok_or_else(|| eyre!("temp file path must include a file name"))?;
        dir.remove_file(file_name).map_err(|err| eyre!(err))?;
        Ok(())
    }

    #[test]
    fn rejects_missing_operation_argument() -> Result<()> {
        let output = run_worker(&[])?;
        ensure!(!output.status.success(), "expected failure status");
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure!(
            stderr.contains("missing operation argument"),
            "expected missing operation argument error"
        );
        Ok(())
    }

    #[test]
    fn rejects_missing_config_argument() -> Result<()> {
        let output = run_worker(&[OsString::from("setup")])?;
        ensure!(!output.status.success(), "expected failure status");
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure!(
            stderr.contains("missing config path argument"),
            "expected missing config path argument error"
        );
        Ok(())
    }

    #[test]
    fn rejects_unknown_operation() -> Result<()> {
        let output = run_worker(&[OsString::from("unknown")])?;
        ensure!(!output.status.success(), "expected failure status");
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure!(
            stderr.contains("unknown pg_worker operation"),
            "expected unknown operation error"
        );
        Ok(())
    }

    #[test]
    fn rejects_extra_arguments() -> Result<()> {
        let output = run_worker(&[
            OsString::from("setup"),
            OsString::from("/tmp/pg_worker_config.json"),
            OsString::from("extra"),
        ])?
        ;
        ensure!(!output.status.success(), "expected failure status");
        let stderr = String::from_utf8_lossy(&output.stderr);
        ensure!(
            stderr.contains("unexpected extra argument"),
            "expected unexpected extra argument error"
        );
        Ok(())
    }

    #[test]
    fn operations_accept_arguments() -> Result<()> {
        let config_path = write_temp_config("not-json")?;
        let operations = ["setup", "start", "stop"];

        for operation in operations {
            let output =
                run_worker(&[OsString::from(operation), config_path.clone().into()])?;
            ensure!(
                !output.status.success(),
                "expected failure status for {operation}"
            );
        }

        remove_temp_file(&config_path)?;
        Ok(())
    }
}
