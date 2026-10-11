use super::{
    managed_files::{self, confined, invalid, read_record, sha256, write_record},
    stage::prepare_game_runtime_stage_at,
    wine::{game_wine_installer_env, game_wine_launch_env},
    GameRuntimeStage,
};
use crate::{
    CommandPlan, GameModuleCatalog, GameProfile, GenericGameProfile, GraphicsBackend, Result,
    SteamIntegration, WineMacDriverConfig, WineSteamRuntime,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BINDING: &str = ".portcellar-installation.toml";
const SESSION: &str = ".portcellar-session.toml";

#[derive(Debug, Clone)]
pub struct InstallationRequest {
    pub id: String,
    pub module_id: String,
    pub variant_id: String,
    pub profile_id: String,
    pub source_root: PathBuf,
    pub wine: PathBuf,
    pub dxmt_root: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    format_version: String,
    id: String,
    module_id: String,
    variant_id: String,
    profile_id: String,
    source_root: PathBuf,
    wine: PathBuf,
    wine_sha256: String,
    wineserver_sha256: String,
    dxmt_root: Option<PathBuf>,
    dxmt_sha256: BTreeMap<String, String>,
    profile: toml::Table,
}

#[derive(Debug)]
pub struct ManagedInstallation {
    root: PathBuf,
    binding: Binding,
    binding_sha256: String,
    profile: GenericGameProfile,
}

impl ManagedInstallation {
    pub fn create(
        root: &Path,
        catalog: &GameModuleCatalog,
        request: &InstallationRequest,
    ) -> Result<Self> {
        macos_execution()?;
        for value in [
            &request.id,
            &request.module_id,
            &request.variant_id,
            &request.profile_id,
        ] {
            slug(value)?;
        }
        if root.file_name().and_then(|s| s.to_str()) != Some(&request.id) {
            return Err(invalid("installation directory must match its id"));
        }
        let parent = root
            .parent()
            .ok_or_else(|| invalid("installation needs an existing parent"))?
            .canonicalize()?;
        reject_managed_path(&parent)?;
        let root = parent.join(&request.id);
        if fs::symlink_metadata(&root).is_ok() {
            return Err(invalid(
                "installation already exists; refusing enrollment or replacement",
            ));
        }
        let profile = catalog
            .profile(&request.module_id, &request.variant_id, &request.profile_id)
            .ok_or_else(|| invalid("module/variant/profile tuple was not found"))?
            .clone();
        validate_managed_profile(&profile)?;
        let source = regular_directory(&request.source_root)?;
        let wine = regular_file(&request.wine)?;
        let wineserver = wine
            .parent()
            .ok_or_else(|| invalid("Wine has no executable directory"))?
            .join("wineserver");
        let dxmt_root = request
            .dxmt_root
            .as_deref()
            .map(regular_directory)
            .transpose()?;
        if root.starts_with(&source) || source.starts_with(&root) || wine.starts_with(&root) {
            return Err(invalid(
                "installation resources must not overlap the source or runtime",
            ));
        }
        let dxmt_sha256 = dxmt_fingerprints(dxmt_root.as_deref(), profile.graphics_backend())?;
        let captured_profile = toml::from_str(&profile.to_profile_toml()?)
            .map_err(|error| invalid(format!("could not capture profile: {error}")))?;
        let binding = Binding {
            format_version: "portcellar-managed-standalone-v1".into(),
            id: request.id.clone(),
            module_id: request.module_id.clone(),
            variant_id: request.variant_id.clone(),
            profile_id: request.profile_id.clone(),
            source_root: source,
            wine_sha256: sha256(&wine)?,
            wineserver_sha256: sha256(&wineserver)?,
            wine,
            dxmt_root,
            dxmt_sha256,
            profile: captured_profile,
        };
        fs::create_dir(&root)?;
        write_record(&root.join(BINDING), &binding)?;
        Self::load(&root)
    }

    pub fn load(root: &Path) -> Result<Self> {
        let root = regular_directory(root)?;
        reject_managed_path(
            root.parent()
                .ok_or_else(|| invalid("installation has no parent"))?,
        )?;
        let binding: Binding = read_record(&root.join(BINDING))?;
        for value in [
            &binding.id,
            &binding.module_id,
            &binding.variant_id,
            &binding.profile_id,
        ] {
            slug(value)?;
        }
        if binding.format_version != "portcellar-managed-standalone-v1"
            || root.file_name().and_then(|s| s.to_str()) != Some(&binding.id)
        {
            return Err(invalid("invalid installation identity or format"));
        }
        let profile_text =
            toml::to_string(&binding.profile).map_err(|error| invalid(error.to_string()))?;
        let profile = crate::games::public_module_profile(&profile_text)?;
        validate_managed_profile(&profile)?;
        if !binding.source_root.is_absolute() || !binding.wine.is_absolute() {
            return Err(invalid("installation inputs must be absolute"));
        }
        let source = regular_directory(&binding.source_root)?;
        if source.starts_with(&root) || root.starts_with(&source) || binding.wine.starts_with(&root)
        {
            return Err(invalid("overlapping installation inputs"));
        }
        let binding_sha256 = sha256(&root.join(BINDING))?;
        let installation = Self {
            root,
            binding,
            binding_sha256,
            profile,
        };
        installation.recheck()?;
        Ok(installation)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn launch_plan(&self) -> Result<CommandPlan> {
        self.recheck()?;
        confined(
            &self.root,
            &format!(
                "stage/game/{}",
                self.profile.windows_exe().replace('\\', "/")
            ),
        )?;
        let mut args = vec![OsString::from(format!(
            "G:\\{}",
            self.profile.windows_exe().replace('/', "\\")
        ))];
        args.extend(
            self.profile
                .launch_arguments()
                .into_iter()
                .map(OsString::from),
        );
        Ok(CommandPlan {
            program: self.binding.wine.clone(),
            args,
            envs: self.environment(true)?,
            current_dir: Some(confined(&self.root, "stage/game")?),
        })
    }

    pub fn installer_plan(&self, installer: &Path, args: &[OsString]) -> Result<CommandPlan> {
        self.selected_installer_plan(installer, args)
            .map(|(plan, _)| plan)
    }

    fn selected_installer_plan(
        &self,
        installer: &Path,
        args: &[OsString],
    ) -> Result<(CommandPlan, String)> {
        self.recheck()?;
        if installer
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case("exe"))
        {
            return Err(invalid("managed installer must be an EXE"));
        }
        let hash = sha256(installer)?;
        let mut arguments = vec![OsString::from(format!(
            "C:\\portcellar\\installers\\{hash}.exe"
        ))];
        arguments.extend_from_slice(args);
        Ok((
            CommandPlan {
                program: self.binding.wine.clone(),
                args: arguments,
                envs: self.environment(false)?,
                current_dir: Some(confined(
                    &self.root,
                    "prefix/drive_c/portcellar/installers",
                )?),
            },
            hash,
        ))
    }

    pub fn prepare(&self) -> Result<()> {
        let _guard = self.acquire()?;
        self.ensure_idle()?;
        self.prepare_stage()
    }

    pub fn launch(&self, timeout: Duration) -> Result<i32> {
        validate_timeout(timeout)?;
        let _guard = self.acquire()?;
        self.ensure_idle()?;
        self.prepare_stage()?;
        self.prepare_prefix(timeout)?;
        self.prepare_dxmt()?;
        self.run_owned(&self.launch_plan()?, timeout)
    }

    pub fn install(&self, installer: &Path, args: &[OsString], timeout: Duration) -> Result<i32> {
        validate_timeout(timeout)?;
        let _guard = self.acquire()?;
        self.ensure_idle()?;
        let (plan, hash) = self.selected_installer_plan(installer, args)?;
        self.prepare_prefix(timeout)?;
        let directory = confined(&self.root, "prefix/drive_c/portcellar/installers")?;
        fs::create_dir_all(&directory)?;
        let target = confined(
            &self.root,
            &format!("prefix/drive_c/portcellar/installers/{hash}.exe"),
        )?;
        if !target.try_exists()? {
            fs::copy(installer, &target)?;
        }
        if sha256(&target)? != hash {
            return Err(invalid(
                "installer changed while copying; copied file retained",
            ));
        }
        self.run_owned(&plan, timeout)
    }

    pub fn replace(
        &self,
        target: &str,
        artifact: &Path,
        original_sha256: Option<&str>,
        replacement_sha256: &str,
    ) -> Result<()> {
        let _guard = self.acquire()?;
        self.ensure_idle()?;
        let game = confined(&self.root, "stage/game")?;
        managed_files::replace(
            &self.root,
            "game",
            &game,
            target,
            artifact,
            original_sha256,
            replacement_sha256,
        )
    }

    pub fn restore(&self, target: &str) -> Result<()> {
        let _guard = self.acquire()?;
        self.ensure_idle()?;
        managed_files::restore(
            &self.root,
            "game",
            &confined(&self.root, "stage/game")?,
            target,
        )
    }

    pub fn recover(&self, timeout: Duration) -> Result<()> {
        validate_timeout(timeout)?;
        let _guard = self.acquire()?;
        self.wait_server(timeout)?;
        let session = confined(&self.root, SESSION)?;
        if session.try_exists()? {
            fs::remove_file(session)?;
            File::open(&self.root)?.sync_all()?;
        }
        Ok(())
    }

    fn recheck(&self) -> Result<()> {
        if regular_directory(&self.root)? != self.root {
            return Err(invalid(
                "installation root changed; refusing aliased resources",
            ));
        }
        if sha256(&self.root.join(BINDING))? != self.binding_sha256 {
            return Err(invalid(
                "installation binding changed; reload it before execution",
            ));
        }
        if sha256(&self.binding.wine)? != self.binding.wine_sha256
            || sha256(&self.binding.wine.parent().unwrap().join("wineserver"))?
                != self.binding.wineserver_sha256
        {
            return Err(invalid(
                "Wine runtime changed; existing installation retained",
            ));
        }
        if dxmt_fingerprints(
            self.binding.dxmt_root.as_deref(),
            self.profile.graphics_backend(),
        )? != self.binding.dxmt_sha256
        {
            return Err(invalid("DXMT payload changed"));
        }
        for child in ["prefix", "stage/game", "home", "logs", "recovery", SESSION] {
            confined(&self.root, child)?;
        }
        Ok(())
    }

    fn acquire(&self) -> Result<File> {
        macos_execution()?;
        self.recheck()?;
        let directory = confined(&self.root, ".portcellar-locks")?;
        fs::create_dir_all(&directory)?;
        let path = confined(&self.root, ".portcellar-locks/installation.lock")?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.try_lock().map_err(|error| {
            invalid(format!("installation busy or locking unavailable: {error}"))
        })?;
        self.recheck()?;
        Ok(file)
    }

    fn ensure_idle(&self) -> Result<()> {
        if confined(&self.root, SESSION)?.try_exists()? {
            return Err(invalid(
                "interrupted or active session retained; use installation recover after Wine stops",
            ));
        }
        self.wait_server(Duration::from_secs(1))
    }

    fn prepare_stage(&self) -> Result<()> {
        let stage = GameRuntimeStage {
            root: confined(&self.root, "stage")?,
            game_root: confined(&self.root, "stage/game")?,
        };
        prepare_game_runtime_stage_at(&self.profile, &self.binding.source_root, &stage)
    }

    fn runtime(&self) -> WineSteamRuntime {
        WineSteamRuntime {
            wine: Some(self.binding.wine.clone()),
            wine_version: None,
            windows_version: Some(self.profile.wine_windows_version().into()),
            dxmt_root: self.binding.dxmt_root.clone(),
            prefix: self.root.join("prefix"),
            steam_exe: None,
            game_manifest: None,
            game_install_dir: Some(self.root.join("stage/game")),
            game_executable: None,
            running: false,
            game_running: false,
            launch_ready_after_login: false,
            readiness_issues: vec![],
            logged_in: None,
            active_session: None,
            cached_credentials: None,
            login_status: None,
            connection_status: None,
            cef_status: None,
            mac_window_status: None,
            virtual_desktop: None,
            mac_driver: WineMacDriverConfig {
                allow_immovable_windows: None,
                retina_mode: None,
            },
        }
    }

    fn environment(&self, game: bool) -> Result<BTreeMap<String, String>> {
        let runtime = self.runtime();
        let mut environment = if game {
            game_wine_launch_env(&runtime, &self.profile)
        } else {
            game_wine_installer_env(&runtime, &self.profile)
        };
        for key in [
            "DYLD_INSERT_LIBRARIES",
            "LD_PRELOAD",
            "WINELOADER",
            "WINEWRAPPER",
            "WINESERVER",
            "PATH",
            "TMP",
            "TEMP",
            "TMPDIR",
            "WINEHOMEDIR",
            "CFFIXED_USER_HOME",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME",
            "XDG_RUNTIME_DIR",
        ] {
            environment.remove(key);
        }
        for (key, relative) in [
            ("WINEPREFIX", "prefix"),
            ("HOME", "home"),
            ("WINEHOMEDIR", "home"),
            ("CFFIXED_USER_HOME", "home"),
            ("TMPDIR", "home/tmp"),
            ("TMP", "home/tmp"),
            ("TEMP", "home/tmp"),
            ("XDG_CACHE_HOME", "home/.cache"),
            ("XDG_CONFIG_HOME", "home/.config"),
            ("XDG_DATA_HOME", "home/.local/share"),
            ("XDG_STATE_HOME", "home/.local/state"),
            ("XDG_RUNTIME_DIR", "home/runtime"),
        ] {
            environment.insert(key.into(), utf8(&confined(&self.root, relative)?)?);
        }
        let bin = self.binding.wine.parent().unwrap();
        environment.insert("WINESERVER".into(), utf8(&bin.join("wineserver"))?);
        environment.insert("PATH".into(), format!("{}:/usr/bin:/bin", utf8(bin)?));
        environment.insert(
            "DYLD_FALLBACK_LIBRARY_PATH".into(),
            utf8(&bin.parent().unwrap().join("lib"))?,
        );
        environment.insert("LANG".into(), "en_US.UTF-8".into());
        if game && self.profile.graphics_backend() == GraphicsBackend::Dxmt {
            let existing = self
                .profile
                .runtime_environment()
                .get("WINEDLLOVERRIDES")
                .cloned()
                .unwrap_or_default();
            environment.insert(
                "WINEDLLOVERRIDES".into(),
                format!("d3d10core,d3d11,dxgi=n,b;winemetal=b;{existing}"),
            );
            environment.remove("WINEDLLPATH_PREPEND");
            environment.remove("DXMT_CONFIG");
            if let Some(config) = self.profile.dxmt_config() {
                environment.insert("DXMT_CONFIG".into(), config.into());
            }
        }
        if let Some(user) = std::env::var_os("USER").and_then(|s| s.into_string().ok()) {
            environment.insert("USER".into(), user.clone());
            environment.insert("LOGNAME".into(), user);
        }
        for (key, value) in &environment {
            if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                return Err(invalid("invalid subprocess environment"));
            }
        }
        Ok(environment)
    }

    fn prepare_prefix(&self, timeout: Duration) -> Result<()> {
        if !confined(&self.root, "prefix/system.reg")?.try_exists()? {
            let plan = CommandPlan {
                program: self.binding.wine.clone(),
                args: vec!["wineboot.exe".into(), "-u".into()],
                envs: self.environment(false)?,
                current_dir: None,
            };
            if self.run_owned(&plan, timeout)? != 0 {
                return Err(invalid("Wine prefix initialization failed; state retained"));
            }
        }
        // Only remove the standard host-root mapping; never follow its destination.
        let z = self.root.join("prefix/dosdevices/z:");
        match fs::symlink_metadata(&z) {
            Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(&z)?,
            Ok(_) => return Err(invalid("unexpected Z: mapping retained")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        #[cfg(unix)]
        for (drive, relative) in [("g:", "stage/game"), ("l:", "logs")] {
            let destination = confined(&self.root, relative)?;
            if !destination.is_dir() {
                continue;
            }
            let directory = confined(&self.root, "prefix/dosdevices")?;
            fs::create_dir_all(&directory)?;
            let link = directory.join(drive);
            match fs::symlink_metadata(&link) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    if fs::read_link(&link)? == destination {
                        continue;
                    }
                    return Err(invalid("existing drive mapping differs; mapping retained"));
                }
                Ok(_) => return Err(invalid("unexpected drive mapping retained")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            std::os::unix::fs::symlink(destination, link)?;
        }
        let plan = CommandPlan {
            program: self.binding.wine.clone(),
            args: vec![
                "reg.exe".into(),
                "add".into(),
                "HKCU\\Software\\Wine".into(),
                "/v".into(),
                "Version".into(),
                "/d".into(),
                self.profile.wine_windows_version().into(),
                "/f".into(),
            ],
            envs: self.environment(false)?,
            current_dir: None,
        };
        if self.run_owned(&plan, timeout)? != 0 {
            return Err(invalid("Wine Windows version configuration failed"));
        }
        for mut plan in super::wine::wine_registry_plans(&self.runtime(), &self.profile)? {
            plan.envs = self.environment(false)?;
            if self.run_owned(&plan, timeout)? != 0 {
                return Err(invalid(
                    "module registry configuration failed; prefix retained",
                ));
            }
        }
        Ok(())
    }

    fn prepare_dxmt(&self) -> Result<()> {
        let Some(payload) = &self.binding.dxmt_root else {
            return Ok(());
        };
        if self.profile.graphics_backend() != GraphicsBackend::Dxmt {
            return Ok(());
        }
        let game = confined(&self.root, "stage/game")?;
        let executable = self.profile.windows_exe().replace('\\', "/");
        let architecture = pe_architecture(&confined(&game, &executable)?)?;
        let directory = Path::new(&executable).parent().unwrap();
        for name in DXMT_LIBRARIES {
            let source = confined(payload, &format!("{architecture}/{name}"))?;
            let relative = directory.join(name);
            let relative = utf8(&relative)?;
            let target = confined(&game, &relative)?;
            let original = confined(&self.binding.source_root, &relative)?;
            let original_sha = if original.try_exists()? {
                Some(sha256(&original)?)
            } else {
                None
            };
            managed_files::replace(
                &self.root,
                "game",
                &game,
                &relative,
                &source,
                original_sha.as_deref(),
                &self.binding.dxmt_sha256[&format!("{architecture}/{name}")],
            )?;
            if !target.is_file() {
                return Err(invalid("DXMT file activation failed"));
            }
        }
        Ok(())
    }

    fn run_owned(&self, plan: &CommandPlan, timeout: Duration) -> Result<i32> {
        validate_timeout(timeout)?;
        for child in [
            "prefix",
            "home/tmp",
            "home/.cache",
            "home/.config",
            "home/.local/share",
            "home/.local/state",
            "home/runtime",
            "logs",
        ] {
            fs::create_dir_all(confined(&self.root, child)?)?;
        }
        let marker = confined(&self.root, SESSION)?;
        write_record(&marker, &BTreeMap::from([("operation", "foreground-wine")]))?;
        let log = confined(&self.root, "logs/wine.log")?;
        let result = run_foreground(plan, timeout, Some(&log));
        // A launcher exiting does not prove its Wine children have stopped.
        self.wait_server(timeout)?;
        fs::remove_file(&marker)?;
        File::open(&self.root)?.sync_all()?;
        result
    }

    fn wait_server(&self, timeout: Duration) -> Result<()> {
        validate_timeout(timeout)?;
        if !self.root.join("prefix").exists() {
            return Ok(());
        }
        let plan = CommandPlan {
            program: self.binding.wine.parent().unwrap().join("wineserver"),
            args: vec!["-w".into()],
            envs: self.environment(false)?,
            current_dir: None,
        };
        let code = run_foreground(&plan, timeout, None)?;
        if code != 0 {
            return Err(invalid("Wine server quiescence could not be confirmed"));
        }
        Ok(())
    }
}

const DXMT_LIBRARIES: [&str; 4] = ["d3d10core.dll", "d3d11.dll", "dxgi.dll", "winemetal.dll"];
fn validate_managed_profile(profile: &GenericGameProfile) -> Result<()> {
    if profile.steam_integration() != SteamIntegration::None
        || !profile.runtime_artifacts().is_empty()
        || profile.windows_install_path_hint().is_some()
        || profile.runtime_options_path_hint().is_some()
    {
        return Err(invalid(
            "managed profile must be standalone and contain no host artifact/options bindings",
        ));
    }
    // shortcut: only explicitly bound runtime components are supported, extend bindings before adding backends/dependencies.
    if !matches!(
        profile.graphics_backend(),
        GraphicsBackend::Auto
            | GraphicsBackend::WineD3D
            | GraphicsBackend::Scgl
            | GraphicsBackend::Dxmt
    ) || !profile.dependencies().is_empty()
    {
        return Err(invalid("managed execution supports auto, wine-d3-d, scgl and dxmt without additional runtime dependencies"));
    }
    Ok(())
}
fn dxmt_fingerprints(
    root: Option<&Path>,
    backend: GraphicsBackend,
) -> Result<BTreeMap<String, String>> {
    if backend == GraphicsBackend::Dxmt && root.is_none() {
        return Err(invalid(
            "DXMT profile requires an explicit local DXMT payload",
        ));
    }
    if backend != GraphicsBackend::Dxmt && root.is_some() {
        return Err(invalid("DXMT payload requires the DXMT backend"));
    }
    let mut hashes = BTreeMap::new();
    if let Some(root) = root {
        for arch in ["x64", "x32"] {
            for name in DXMT_LIBRARIES {
                let path = format!("{arch}/{name}");
                hashes.insert(path.clone(), sha256(&confined(root, &path)?)?);
            }
        }
    }
    Ok(hashes)
}

fn pe_architecture(path: &Path) -> Result<&'static str> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = File::open(path)?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header)?;
    if &header[..2] != b"MZ" {
        return Err(invalid("DXMT selection requires a PE executable"));
    }
    file.seek(SeekFrom::Start(
        u32::from_le_bytes(header[60..64].try_into().unwrap()).into(),
    ))?;
    let mut pe = [0u8; 6];
    file.read_exact(&mut pe)?;
    if &pe[..4] != b"PE\0\0" {
        return Err(invalid("invalid PE header"));
    }
    match u16::from_le_bytes([pe[4], pe[5]]) {
        0x8664 => Ok("x64"),
        0x14c => Ok("x32"),
        _ => Err(invalid("unsupported DXMT executable architecture")),
    }
}

fn run_foreground(plan: &CommandPlan, timeout: Duration, log: Option<&Path>) -> Result<i32> {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    let mut command = Command::new(&plan.program);
    command
        .env_clear()
        .envs(&plan.envs)
        .args(&plan.args)
        .stdin(Stdio::null());
    if let Some(log) = log {
        let output = OpenOptions::new().create(true).append(true).open(log)?;
        command.stdout(output.try_clone()?).stderr(output);
    }
    if let Some(directory) = &plan.current_dir {
        command.current_dir(directory);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return status.code().ok_or_else(|| {
                #[cfg(unix)]
                if let Some(signal) = status.signal() {
                    return invalid(format!("managed command terminated by signal {signal}"));
                }
                invalid("managed command terminated without an exit code")
            });
        }
        if start.elapsed() >= timeout {
            child.kill()?;
            child.wait()?;
            return Err(invalid("managed command timed out; child processes may remain; session state retained until quiescence"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn regular_file(path: &Path) -> Result<PathBuf> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("expected a regular local file"));
    }
    Ok(path.canonicalize()?)
}
fn regular_directory(path: &Path) -> Result<PathBuf> {
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(invalid("expected a real local directory"));
    }
    Ok(path.canonicalize()?)
}
fn utf8(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| invalid("managed paths must support UTF-8"))
}
fn slug(value: &str) -> Result<()> {
    if value.is_empty()
        || value.split('-').any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(invalid("invalid installation identifier"));
    }
    Ok(())
}
fn validate_timeout(timeout: Duration) -> Result<()> {
    if timeout.is_zero() || timeout > Duration::from_secs(86400) {
        return Err(invalid(
            "timeout must be positive and at most 86400 seconds",
        ));
    }
    Ok(())
}
fn macos_execution() -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(invalid("managed execution currently requires macOS"));
    }
    Ok(())
}

pub(crate) fn reject_managed_path(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let refuse_owned_ancestor = |path: &Path| -> Result<()> {
        for ancestor in path.ancestors() {
            match fs::symlink_metadata(ancestor.join(BINDING)) {
                Ok(_) => return Err(invalid(
                    "managed resource requires the installation owner; legacy execution refused",
                )),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                    ) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    };
    refuse_owned_ancestor(&absolute)?;
    let mut existing = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(existing) {
            Ok(_) => break,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                suffix.push(
                    existing
                        .file_name()
                        .ok_or_else(|| invalid("path has no existing ancestor"))?,
                );
                existing = existing
                    .parent()
                    .ok_or_else(|| invalid("path has no existing ancestor"))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut resolved = existing.canonicalize()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    refuse_owned_ancestor(&resolved)
}
