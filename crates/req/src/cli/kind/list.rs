use crate::cli::terminal::Colorize;

pub fn run(config_path: &std::path::Path) -> anyhow::Result<()> {
    let config = if config_path.exists() {
        requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        requiem_core::Config::default()
    };

    let kinds = config.allowed_kinds();

    if kinds.is_empty() {
        println!("{}", "No kinds configured (all kinds allowed)".dim());
    } else {
        println!("Registered requirement kinds:");
        for kind in kinds {
            println!("  • {kind}");
        }
    }

    Ok(())
}
