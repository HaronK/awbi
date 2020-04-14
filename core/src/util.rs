use anyhow::Result;
use std::path::PathBuf;

pub(crate) fn data_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_exe()?;

    // Go to project folder
    dir.pop();
    dir.pop();
    dir.pop();
    dir.pop();

    dir.push("data");

    Ok(dir)
}
