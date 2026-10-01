use color_eyre::eyre::Result;
use harvest_claude::layout::persistence;
use harvest_claude::logging;

fn main() -> Result<()> {
    // The guard flushes buffered log lines when `main` returns.
    let _log_guard = persistence::config_dir()
        .and_then(|dir| logging::init(&dir))
        .inspect_err(|e| eprintln!("logging disabled: {e:#}"))
        .ok();
    harvest_claude::run()?;
    Ok(())
}
