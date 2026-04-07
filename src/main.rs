use color_eyre::eyre::Result;

fn main() -> Result<()> {
    harvest_claude::run()?;
    Ok(())
}
