use anyhow::Result;

use crate::config::{self, LlmConfig};

pub fn run(action: &crate::cli::ProviderAction) -> Result<()> {
    let mut cfg = config::load()?;

    match action {
        crate::cli::ProviderAction::List => {
            let active = cfg
                .active_provider
                .as_deref()
                .unwrap_or("(default [llm] block)");
            println!("Active: {active}");
            println!();
            if cfg.providers.is_empty() {
                println!("No named profiles. Add one with:");
                println!("  sara provider add <name> --type <azure|openai|mlx|ollama> ...");
            } else {
                let mut names: Vec<_> = cfg.providers.keys().collect();
                names.sort();
                for name in names {
                    let p = &cfg.providers[name];
                    let marker = if cfg.active_provider.as_deref() == Some(name) {
                        "▶ "
                    } else {
                        "  "
                    };
                    println!("{marker}{name}  ({} / {})", p.provider, p.model);
                }
            }
        }

        crate::cli::ProviderAction::Use { name } => {
            if name == "default" {
                cfg.active_provider = None;
                config::save(&cfg)?;
                println!("Switched to default [llm] block.");
            } else if cfg.providers.contains_key(name) {
                cfg.active_provider = Some(name.clone());
                config::save(&cfg)?;
                let p = &cfg.providers[name];
                println!("Switched to '{name}' ({} / {})", p.provider, p.model);
            } else {
                anyhow::bail!(
                    "No profile named '{name}'. Run `sara provider list` to see available profiles."
                );
            }
        }

        crate::cli::ProviderAction::Add {
            name,
            provider_type,
            model,
            url,
            key,
        } => {
            let llm = LlmConfig {
                provider: provider_type.clone(),
                model: model.clone(),
                base_url: url.clone(),
                api_key: key.clone(),
                timeout_secs: 60,
            };
            cfg.providers.insert(name.clone(), llm);
            // Auto-activate the new profile
            cfg.active_provider = Some(name.clone());
            config::save(&cfg)?;
            println!("Added profile '{name}' and set it as active.");
            println!("  provider : {provider_type}");
            println!("  model    : {model}");
            if let Some(u) = url {
                println!("  url      : {u}");
            }
        }

        crate::cli::ProviderAction::Remove { name } => {
            if cfg.providers.remove(name).is_none() {
                anyhow::bail!("No profile named '{name}'.");
            }
            if cfg.active_provider.as_deref() == Some(name) {
                cfg.active_provider = None;
                println!("Removed '{name}' and reverted to default [llm] block.");
            } else {
                println!("Removed profile '{name}'.");
            }
            config::save(&cfg)?;
        }
    }

    Ok(())
}
