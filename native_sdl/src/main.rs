use anyhow::Result;
use awbi_core::{engine::Engine, reference::Ref, system::System};
use sdl_system::SdlSystem;
use std::path::PathBuf;

mod sdl_system;

fn proj_dir() -> Result<PathBuf> {
    let mut dir = std::env::current_exe()?;

    // Go to project folder
    dir.pop();
    dir.pop();
    dir.pop();
    dir.pop();

    Ok(dir)
}

fn data_dir() -> Result<PathBuf> {
    let mut dir = proj_dir()?;

    dir.push("data");

    Ok(dir)
}

fn main() -> Result<()> {
    let data_dir = data_dir()?;
    let sys: Ref<Box<(dyn System)>> = Ref::new(Box::new(SdlSystem::new()?));
    let mut engine = Engine::new(sys, data_dir.to_str().unwrap(), data_dir.to_str().unwrap());

    engine.init()?;
    // println!("=== Engine State ===\n{:#?}=== Engine State ===", engine);
    engine.run()
}
