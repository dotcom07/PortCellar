use portcellar_core::{GameModuleCatalog, GameProfile, GenericGameProfile, SteamIntegration};
use std::{fs, path::PathBuf};

const DESCRIPTOR: &str = r#"format_version = "portcellar-module-v1"
id = "example"
name = "Example"
revision = 1
[[variants]]
id = "windows"
name = "Windows"
[[variants.profiles]]
id = "default"
path = "profiles/game.toml"
"#;

#[test]
fn catalog_validates_identity_profiles_and_untrusted_paths() {
    let root = std::env::temp_dir().join(format!("portcellar-modules-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let module = root.join("example");
    fs::create_dir_all(module.join("profiles")).unwrap();
    let profile = GenericGameProfile::new("local-example", "Example", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .to_profile_toml()
        .unwrap();
    let load = |descriptor: &str, profile_text: &str| {
        fs::write(module.join("module.toml"), descriptor).unwrap();
        fs::write(module.join("profiles/game.toml"), profile_text).unwrap();
        GameModuleCatalog::load(&root)
    };
    let catalog = load(DESCRIPTOR, &profile).unwrap();
    assert_eq!(catalog.modules.len(), 1);
    assert_eq!(
        catalog
            .profile("example", "windows", "default")
            .unwrap()
            .app_id(),
        "local-example"
    );
    assert!(catalog.profile("example", "missing", "default").is_none());
    assert!(load(
        &DESCRIPTOR.replace("Example", "\u{30b2}\u{30fc}\u{30e0}"),
        &profile
    )
    .is_ok());
    let second_variant = DESCRIPTOR.split_once("[[variants]]").unwrap().1;
    let two_variants = format!(
        "{DESCRIPTOR}[[variants]]{}",
        second_variant.replace("id = \"windows\"", "id = \"other\"")
    );
    assert!(load(&two_variants, &profile)
        .unwrap()
        .profile("example", "other", "default")
        .is_some());
    let duplicate_variant = format!("{DESCRIPTOR}[[variants]]{second_variant}");
    assert!(load(&duplicate_variant, &profile)
        .unwrap_err()
        .to_string()
        .contains("unique id"));
    for descriptor in [
        DESCRIPTOR.replace("portcellar-module-v1", "future"),
        DESCRIPTOR.replace("revision = 1", "revision = 0"),
        DESCRIPTOR.replace("revision = 1", "revision = 1\nunknown = true"),
        DESCRIPTOR.replace("revision = 1", "revision = 1\nrevision = 2"),
        DESCRIPTOR.replace("id = \"example\"", "id = \"wrong\""),
        DESCRIPTOR.replace("id = \"windows\"", "id = \"windows.1\""),
        DESCRIPTOR.replace("name = \"Windows\"", "name = \"Windows\"\nunknown = true"),
        format!("{DESCRIPTOR}unknown = true\n"),
        format!(
            "{DESCRIPTOR}[[variants.profiles]]\nid = \"default\"\npath = \"profiles/game.toml\"\n"
        ),
        format!(
            "{DESCRIPTOR}[[variants]]\nid = \"windows\"\nname = \"Duplicate\"\nprofiles = []\n"
        ),
        DESCRIPTOR.replace(
            "[[variants.profiles]]",
            "stores = {steam = \"abc\"}\n[[variants.profiles]]",
        ),
    ] {
        assert!(
            load(&descriptor, &profile).is_err(),
            "accepted {descriptor}"
        );
    }
    for path in [
        "../outside.toml",
        "/outside.toml",
        "C:/outside.toml",
        "profiles//game.toml",
        "./profiles/game.toml",
        "profiles/../profiles/game.toml",
        "profiles/missing.toml",
        "profiles/game.txt",
        "",
    ] {
        assert!(
            load(&DESCRIPTOR.replace("profiles/game.toml", path), &profile).is_err(),
            "accepted {path}"
        );
    }
    for extra in ["unknown = true\n", "windows_install_path = 'Z:\\private'\n", "wine_engine_path = '/private/wine'\n", "[[binary_patches]]\ntarget_path = 'Game.exe'\nkind = 'large-address-aware'\nunknown = true\n", "[[runtime_artifacts]]\nid = 'dll'\nsource_path = '/private/dll'\ntarget_path = 'dll'\n"] {
        let mut document: toml::Table = toml::from_str(&profile).unwrap();
        document.extend(toml::from_str::<toml::Table>(extra).unwrap());
        let text = toml::to_string(&document).unwrap();
        // Ensure rejection comes from the module boundary, not malformed TOML
        // or a missing required profile field.
        GenericGameProfile::from_profile_toml(&text).unwrap();
        assert!(load(DESCRIPTOR, &text).is_err(), "accepted {extra}");
    }
    let dynamic = GenericGameProfile::new("local-example", "Example", "Game.exe")
        .with_steam_integration(SteamIntegration::None)
        .with_runtime_options_path(r"C:\Games\Example\options.ini")
        .with_runtime_option("ArbitraryOption", "1")
        .with_runtime_environment("EXAMPLE_VARIABLE", "value")
        .with_binary_patch(portcellar_core::GameBinaryPatch {
            target_path: "Game.exe".into(),
            kind: portcellar_core::GameBinaryPatchKind::LargeAddressAware,
        })
        .to_profile_toml()
        .unwrap();
    assert!(load(DESCRIPTOR, &dynamic).is_ok());
    let unknown = format!("unknown = true\n{profile}");
    assert!(GenericGameProfile::from_profile_toml(&unknown).is_ok());
    let steam = GenericGameProfile::new("250900", "Example", "Game.exe")
        .to_profile_toml()
        .unwrap();
    assert!(load(DESCRIPTOR, &steam).is_err());
    let with_store = DESCRIPTOR.replace(
        "[[variants.profiles]]",
        "stores = {steam = \"250900\"}\n[[variants.profiles]]",
    );
    assert!(load(&with_store, &steam).is_ok());
    assert!(load(&with_store.replace("250900", "10"), &steam).is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        load(DESCRIPTOR, &profile).unwrap();
        fs::rename(module.join("profiles"), module.join("real-profiles")).unwrap();
        symlink("real-profiles", module.join("profiles")).unwrap();
        assert!(GameModuleCatalog::load(&root).is_err());
        fs::remove_file(module.join("profiles")).unwrap();
        fs::rename(module.join("real-profiles"), module.join("profiles")).unwrap();
        fs::rename(
            module.join("profiles/game.toml"),
            module.join("profiles/real.toml"),
        )
        .unwrap();
        symlink("real.toml", module.join("profiles/game.toml")).unwrap();
        assert!(GameModuleCatalog::load(&root).is_err());
        fs::remove_file(module.join("profiles/game.toml")).unwrap();
        fs::rename(
            module.join("profiles/real.toml"),
            module.join("profiles/game.toml"),
        )
        .unwrap();
        let outside = root.with_extension("outside");
        fs::rename(&module, &outside).unwrap();
        symlink(&outside, &module).unwrap();
        assert!(GameModuleCatalog::load(&root)
            .unwrap_err()
            .to_string()
            .contains("symlink"));
        fs::remove_dir_all(outside).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_catalog_loads_both_games_without_private_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules");
    let catalog = GameModuleCatalog::load(&root).unwrap();
    assert!(catalog
        .profile("simcity-4", "windows-1-1-610", "scgl")
        .is_some());
    assert_eq!(
        catalog
            .profile("binding-of-isaac-rebirth", "windows-steam", "wine-steam")
            .unwrap()
            .app_id(),
        "250900"
    );
}
