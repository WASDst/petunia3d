//! DesktopHost e infraestrutura de plataforma para a Application.
//!
//! Website/docs/02-application/commands-events-services.md:
//! 1. Estado de plataforma sai da Application: janela, relógio do sistema, filesystem, locks, watchers.
//! 2. DesktopHost traduz eventos de plataforma para contratos neutros da Application.
//! 3. Construtores da Application são 100% puros (não leem variáveis de ambiente, não consultam relógio, não tocam filesystem).
//! 4. Injeção explícita de Clock, AutosaveStorage e SessionLock.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use petunia_core::{AppState, Clock, SystemClock};
use petunia_project::SessionLockInfo;

use crate::{Application, ApplicationConfig};

/// Guarda RAII para o marcador de sessão (`session.lock`).
///
/// Remove automaticamente o arquivo de lock ao ser dropado caso pertença ao processo atual.
#[derive(Debug)]
pub struct SessionLockGuard {
    lock_path: PathBuf,
    info: SessionLockInfo,
    active: bool,
}

impl SessionLockGuard {
    /// Cria uma nova guarda ativa para o caminho e informações informadas.
    pub fn new(lock_path: PathBuf, info: SessionLockInfo) -> Self {
        Self {
            lock_path,
            info,
            active: true,
        }
    }

    /// Informações registradas no lock de sessão.
    pub fn info(&self) -> &SessionLockInfo {
        &self.info
    }

    /// Caminho do arquivo de lock.
    pub fn path(&self) -> &Path {
        &self.lock_path
    }

    /// Libera explicitamente o lock, removendo o arquivo do disco.
    pub fn release(mut self) -> Result<(), std::io::Error> {
        self.active = false;
        if self.lock_path.exists() {
            fs::remove_file(&self.lock_path)?;
        }
        Ok(())
    }
}

impl Drop for SessionLockGuard {
    fn drop(&mut self) {
        if self.active && self.lock_path.exists() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

/// Host da plataforma desktop para orquestração da Application neutra.
pub struct DesktopHost;

impl DesktopHost {
    /// Cria uma nova instância de `Application` neutra com relógio injetado explicitamente.
    ///
    /// Garante pureza total: nenhuma leitura de variáveis de ambiente, criação de locks
    /// ou I/O de disco é realizada durante a construção.
    pub fn create_application(
        config: ApplicationConfig,
        clock: Option<Arc<dyn Clock>>,
    ) -> Application {
        let mut state = AppState::new(&config.language);
        if let Some(clock) = clock {
            state.clock = clock;
        } else {
            state.clock = Arc::new(SystemClock);
        }
        Application::from_state(state)
    }

    /// Aplica preferências de usuário do desktop ao estado da aplicação.
    pub fn apply_preferences(state: &mut AppState, preferences: &petunia_config::UserPreferences) {
        if let Some(language) = &preferences.language {
            state.ui.i18n.set_lang(language);
        }
        state.ui.invert_vertical_drag = preferences.invert_vertical_drag;
        state.ui.colorblind_axes = preferences.colorblind_axes;
        state.ui.reduced_motion = preferences.reduced_motion;
        state.ui.multiselection_measure_tag = preferences.multiselection_measure_tag;
        state.ui.selection_rgb = preferences.selection_rgb;
        state.ui.selection_thickness = preferences.selection_thickness.clamp(1.0, 6.0);
        state.ui.gizmo_size = preferences.gizmo_size.clamp(48.0, 160.0);
        state.ui.gizmo_thickness = preferences.gizmo_thickness.clamp(1.0, 6.0);
        state.ui.select_gizmo_handles = preferences.select_gizmo_handles;
        state.ui.nav_cube_opacity = preferences.nav_cube_opacity.clamp(0.2, 1.0);
        state.ui.model_quick_actions = preferences.model_quick_actions.clone();
        let grid = preferences.viewport_grid.sanitized();
        state.session.grid_settings.size = grid.size;
        state.session.grid_settings.spacing = grid.spacing;
        state.session.grid_settings.opacity = grid.opacity;
        state.session.grid_settings.axes = grid.axes;
        if !preferences.theme_id.is_empty() {
            state.ui.active_theme_id = preferences.theme_id.clone();
        }
        if !preferences.active_keymap_id.is_empty() {
            state.ui.active_keymap_id = preferences.active_keymap_id.clone();
            state.ui.keybinds =
                petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
        }
    }

    /// Inspeciona o arquivo `session.lock` em um diretório de projeto ou recuperação.
    pub fn check_session_lock(dir: &Path) -> Option<SessionLockInfo> {
        let lock_path = dir.join("session.lock");
        if !lock_path.exists() {
            return None;
        }
        let bytes = fs::read(&lock_path).ok()?;
        postcard::from_bytes(&bytes).ok()
    }

    /// Adquire o lock de sessão gravando `session.lock` no diretório de trabalho.
    ///
    /// Se já existir um lock ativo de outro processo em execução, retorna erro.
    pub fn acquire_session_lock(
        dir: &Path,
        project_name: &str,
        project_path: Option<&Path>,
        now_secs: u64,
    ) -> Result<SessionLockGuard, std::io::Error> {
        let lock_path = dir.join("session.lock");
        if let Some(existing) = Self::check_session_lock(dir)
            && Self::is_process_alive(existing.pid)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "Session locked by active process PID {} for project '{}'",
                    existing.pid, existing.project_name
                ),
            ));
        }

        let info = SessionLockInfo {
            pid: std::process::id(),
            start_time: now_secs,
            project_name: project_name.to_string(),
            project_path: project_path.map(|p| p.to_string_lossy().into_owned()),
        };

        let encoded = postcard::to_allocvec(&info).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to encode session lock: {e}"),
            )
        })?;

        fs::write(&lock_path, encoded)?;
        Ok(SessionLockGuard::new(lock_path, info))
    }

    /// Verifica se um processo com o PID fornecido ainda está em execução no sistema.
    pub fn is_process_alive(pid: u32) -> bool {
        if pid == 0 {
            return false;
        }
        #[cfg(target_os = "linux")]
        {
            Path::new("/proc").join(pid.to_string()).exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}
