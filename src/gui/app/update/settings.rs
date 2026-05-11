use super::super::{App, Message};
use crate::config::AppConfigUpdates;
use crate::gui::settings::SettingsDraft;
use crate::terminal::TerminalTheme;
use iced::{Size, Task, window};

impl App {
    pub(super) fn apply_settings(&mut self, save: bool) -> Task<Message> {
        let updates = self.settings_draft.to_updates();

        #[cfg(target_os = "macos")]
        {
            let blur_toggled = updates
                .blur_enabled
                .is_some_and(|v| v != self.config.theme.blur_enabled);
            let radius_changed = updates
                .macos_blur_radius
                .is_some_and(|v| v != self.config.theme.macos_blur_radius);
            if blur_toggled || radius_changed {
                self.show_restart_confirm = true;
                self.pending_settings_updates = Some(updates);
                self.pending_save_on_restart = save;
                return Task::none();
            }
        }

        let resize_task = self.apply_updates_to_runtime(updates);

        if save {
            self.queue_config_save();
        }

        resize_task
    }

    pub(super) fn queue_config_save(&self) {
        let _ = self.config_save_tx.send(self.config.clone());
    }

    pub(super) fn apply_updates_to_runtime(&mut self, updates: AppConfigUpdates) -> Task<Message> {
        let affects_locale = updates.language.is_some();
        let affects_theme = updates.color_scheme.is_some()
            || updates.foreground.is_some()
            || updates.background.is_some()
            || updates.cursor.is_some()
            || updates.ansi_colors.is_some()
            || updates.background_opacity.is_some()
            || updates.blur_enabled.is_some()
            || updates.macos_blur_radius.is_some();
        let affects_grid = updates.window_width.is_some()
            || updates.window_height.is_some()
            || updates.terminal_font_selection.is_some()
            || updates.terminal_font_size.is_some()
            || updates.terminal_padding_x.is_some()
            || updates.terminal_padding_y.is_some();
        let affects_window = updates.window_width.is_some() || updates.window_height.is_some();

        self.config.apply_updates(updates);
        self.settings_draft = SettingsDraft::from_config(&self.config);

        if affects_locale {
            crate::i18n::set_locale(self.config.ui.language.as_deref());
        }
        if affects_theme {
            self.palette = crate::gui::theme::Palette::from_theme(&self.config.theme);
        }

        let resize_task = if affects_window {
            let new_size = Size::new(self.config.ui.window_width, self.config.ui.window_height);
            if (self.window_size.width - new_size.width).abs() > f32::EPSILON
                || (self.window_size.height - new_size.height).abs() > f32::EPSILON
            {
                self.window_size = new_size;
                window::latest().and_then(move |id| window::resize(id, new_size))
            } else {
                Task::none()
            }
        } else {
            Task::none()
        };

        if affects_grid || affects_theme {
            let (cols, rows) = self.grid_for_size(self.window_size);
            let theme = affects_theme.then(|| TerminalTheme::from_config(&self.config));
            for tab in &mut self.tabs {
                if affects_grid {
                    let current = tab.size();
                    if current.columns != cols || current.lines != rows {
                        tab.resize(cols, rows);
                    }
                }
                if let Some(ref theme) = theme {
                    tab.set_theme(theme.clone());
                }
            }
        }

        resize_task
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_confirm_restart(&mut self) -> Task<Message> {
        if let Some(updates) = self.pending_settings_updates.take() {
            let _ = self.apply_updates_to_runtime(updates);
            if self.pending_save_on_restart
                && let Err(err) = self.config.save()
            {
                eprintln!("Failed to save config: {err}");
            }
        }

        let restart_spawned = match std::env::current_exe() {
            Ok(current_exe) => {
                let args: Vec<_> = std::env::args_os().skip(1).collect();
                match std::process::Command::new(current_exe).args(args).spawn() {
                    Ok(_) => true,
                    Err(err) => {
                        eprintln!("Failed to relaunch app: {err}");
                        false
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to locate executable for restart: {err}");
                false
            }
        };

        self.show_restart_confirm = false;
        self.pending_save_on_restart = false;

        if restart_spawned {
            return iced::exit();
        }

        Task::none()
    }
}
