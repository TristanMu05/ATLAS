fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _result = atlas_logger::simulation(|_update| Ok(()))?;
    Ok(())
}