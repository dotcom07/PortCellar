use portcellar_core::{GameModuleCatalog, PortCellarError, Result};
use std::path::PathBuf;

pub(crate) fn inspect(args: &[String], single: bool) -> Result<()> {
    let mut root = PathBuf::from("modules");
    let mut id = None;
    let mut options = args.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--root" => {
                root = PathBuf::from(
                    options
                        .next()
                        .ok_or_else(|| error("--root requires a path"))?,
                )
            }
            "--id" if single => {
                id = Some(
                    options
                        .next()
                        .ok_or_else(|| error("--id requires a module id"))?,
                )
            }
            _ => return Err(error(&format!("unknown module option: {option}"))),
        }
    }
    if single && id.is_none() {
        return Err(error("game module requires --id"));
    }
    let catalog = GameModuleCatalog::load(&root)?;
    if let Some(id) = id {
        if !catalog
            .modules
            .iter()
            .any(|entry| &entry.descriptor.id == id)
        {
            return Err(error(&format!("module not found: {id}")));
        }
    }
    for module in &catalog.modules {
        let descriptor = &module.descriptor;
        if id.is_some_and(|id| id != &descriptor.id) {
            continue;
        }
        println!(
            "{}\t{}\trevision {}",
            descriptor.id, descriptor.name, descriptor.revision
        );
        for variant in &descriptor.variants {
            for profile in &variant.profiles {
                println!(
                    "  {}/{}/{}\t{}",
                    descriptor.id, variant.id, profile.id, profile.path
                );
            }
        }
    }
    Ok(())
}

fn error(message: &str) -> PortCellarError {
    PortCellarError::Message(message.to_string())
}
