use crate::cli::terminal::Colorize;

pub fn run(config_path: &std::path::Path) -> anyhow::Result<()> {
    let config = if config_path.exists() {
        requiem_core::Config::load(config_path).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        requiem_core::Config::default()
    };

    let allowed_kinds = config.allowed_kinds();
    let metadata = config.kind_metadata();
    let kinds: Vec<String> = if allowed_kinds.is_empty() {
        let mut keys: Vec<String> = metadata.keys().cloned().collect();
        keys.sort_unstable();
        keys
    } else {
        allowed_kinds.to_vec()
    };

    if kinds.is_empty() {
        println!("{}", "No kinds configured (all kinds allowed)".dim());
    } else {
        println!("Registered requirement kinds:");
        for kind in kinds {
            println!("  • {kind}");

            if let Some(meta) = metadata.get(&kind) {
                if let Some(description) = &meta.description {
                    println!("     {description}");
                }
            }
        }
    }

    Ok(())
}
