mod settings;
mod tab;
mod terminal;

use super::{App, Message, SETTINGS_TAB_INDEX};
use crate::gui::settings::{SettingsDraft, SettingsField};
use crate::gui::tab::ShellKind;
use iced::keyboard::{Key, key::Named};
use iced::time::Instant;
use iced::{Task, widget};
use std::sync::LazyLock;

pub(in crate::gui) static TAB_BAR_SCROLLABLE_ID: LazyLock<widget::Id> =
    LazyLock::new(widget::Id::unique);
pub(in crate::gui) static TERMINAL_SCROLLABLE_ID: LazyLock<widget::Id> =
    LazyLock::new(widget::Id::unique);

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Tab management ──────────────────────────────────────
            Message::TabSelected(index) => {
                if index == SETTINGS_TAB_INDEX && self.settings_open {
                    self.active_tab = SETTINGS_TAB_INDEX;
                } else if index < self.tabs.len() {
                    self.active_tab = index;
                    self.dragging_tab = Some(index);
                    self.drag_target = None;
                }
            }
            Message::TabDragHover(index) => {
                if self.dragging_tab.is_some() && index < self.tabs.len() {
                    self.drag_target = Some(index);
                }
            }
            Message::TabDragRelease => {
                if let Some(from) = self.dragging_tab.take()
                    && let Some(target) = self.drag_target.take()
                    && from != target
                    && from < self.tabs.len()
                    && target < self.tabs.len()
                {
                    let tab = self.tabs.remove(from);
                    self.tabs.insert(target, tab);
                    if self.active_tab == from {
                        self.active_tab = target;
                    } else if from < self.active_tab && target >= self.active_tab {
                        self.active_tab -= 1;
                    } else if from > self.active_tab && target <= self.active_tab {
                        self.active_tab += 1;
                    }
                }
                self.drag_target = None;
            }
            Message::CloseTab(index) => {
                self.handle_close_tab(index);
            }
            Message::OpenShellPicker => {
                self.show_shell_picker = true;
                self.shell_picker_selected = 0;
                self.shell_picker_anim.go_mut(true, Instant::now());
            }
            Message::CloseShellPicker => {
                self.shell_picker_anim.go_mut(false, Instant::now());
            }
            Message::CreateTab(shell) => {
                return self.create_tab(shell);
            }
            Message::CreateSshTab(profile_index) => {
                if let Some(profile) = self.config.ssh_profiles.get(profile_index) {
                    let shell = ShellKind::Ssh(profile.clone());
                    return self.create_tab(shell);
                }
            }

            // ── Settings ────────────────────────────────────────────
            Message::AddSshProfile => {
                self.config
                    .ssh_profiles
                    .push(crate::config::SshProfile::default());
                self.settings_draft = SettingsDraft::from_config(&self.config);
            }
            Message::RemoveSshProfile(index) => {
                if index < self.config.ssh_profiles.len() {
                    self.config.ssh_profiles.remove(index);
                    self.settings_draft = SettingsDraft::from_config(&self.config);
                }
            }
            Message::SshProfileFieldChanged(index, field, value) => {
                self.settings_draft.update_ssh_profile(index, field, value);
            }
            Message::SaveSshProfiles => {
                self.settings_draft
                    .apply_ssh_profiles_to(&mut self.config.ssh_profiles);
                // Save passwords to OS keychain (not in config file)
                for profile in &self.config.ssh_profiles {
                    if let Some(ref pw) = profile.password {
                        crate::keychain::set_password(&profile.host, &profile.user, pw);
                    } else {
                        crate::keychain::delete_password(&profile.host, &profile.user);
                    }
                }
                if let Err(err) = self.config.save() {
                    eprintln!("Failed to save config: {err}");
                }
                self.settings_draft = SettingsDraft::from_config(&self.config);
            }
            Message::OpenSettingsTab => {
                self.settings_open = true;
                self.active_tab = SETTINGS_TAB_INDEX;
                self.settings_draft = SettingsDraft::from_config(&self.config);
            }
            Message::SelectSettingsCategory(category) => {
                self.settings_category = category;
                if !self.settings_open {
                    self.settings_open = true;
                    self.active_tab = SETTINGS_TAB_INDEX;
                    self.settings_draft = SettingsDraft::from_config(&self.config);
                }
            }
            Message::SettingsInputChanged(field, value) => {
                self.settings_draft.update(field, value);
            }
            Message::SettingsBlurToggled(enabled) => {
                self.settings_draft.blur_enabled = enabled;
            }
            Message::FontSelected(option) => {
                self.settings_draft
                    .update(SettingsField::TerminalFontSelection, option.value);
            }
            Message::ToggleShowAllFonts(show_all) => {
                self.show_all_fonts = show_all;
                self.font_combo_state = super::build_font_combo_state(
                    &self.all_font_options,
                    show_all,
                    self.config.terminal.font_selection.as_deref(),
                );
            }
            Message::ApplySettings => {
                return self.apply_settings(false);
            }
            Message::SaveSettings => {
                return self.apply_settings(true);
            }
            #[cfg(target_os = "macos")]
            Message::ConfirmRestartForBlur => {
                return self.handle_confirm_restart();
            }
            #[cfg(target_os = "macos")]
            Message::CancelRestartForBlur => {
                self.show_restart_confirm = false;
                self.pending_settings_updates = None;
                self.pending_save_on_restart = false;
            }

            // ── Terminal / PTY ──────────────────────────────────────
            Message::PtySenderReady(sender) => {
                self.pty_sender = Some(sender);
            }
            Message::PtyOutput(event) => {
                self.handle_pty_event(event);
                self.ignore_scrollable_sync = true;
                return self.sync_terminal_scrollable();
            }
            Message::PtyOutputBatch(events) => {
                for event in events {
                    self.handle_pty_event(event);
                }
                self.ignore_scrollable_sync = true;
                return self.sync_terminal_scrollable();
            }
            Message::KeyPressed {
                key,
                modifiers,
                text,
            } => {
                return self.handle_key_pressed(key, modifiers, text);
            }
            Message::TabBarScroll(delta) => {
                return self.handle_tab_bar_scroll(delta);
            }
            Message::TabBarScrolled(x) => {
                self.tab_bar_scroll_x = x;
            }
            Message::SelectionChanged(sel) => {
                if self.active_tab != SETTINGS_TAB_INDEX
                    && let Some(tab) = self.tabs.get_mut(self.active_tab)
                {
                    tab.selection = sel;
                }
            }
            Message::TerminalMousePress { col, row } => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    tab.send_mouse_event(0, col, row, true);
                }
            }
            Message::TerminalMouseRelease { col, row } => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    tab.send_mouse_event(0, col, row, false);
                }
            }
            Message::TerminalMouseDrag { col, row } => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    // Button 0 + 32 = motion flag for SGR drag reporting
                    tab.send_mouse_event(32, col, row, true);
                }
            }
            Message::PasteClipboard(text) => {
                if !text.is_empty()
                    && self.active_tab != SETTINGS_TAB_INDEX
                    && let Some(tab) = self.tabs.get_mut(self.active_tab)
                    && let crate::gui::tab::TerminalSession::Active(session) = &tab.session
                {
                    let _ = session.send_bytes(text.as_bytes());
                }
            }
            Message::TerminalScroll(rel_y) => {
                if self.ignore_scrollable_sync {
                    self.ignore_scrollable_sync = false;
                } else if self.active_tab != SETTINGS_TAB_INDEX
                    && let Some(tab) = self.tabs.get_mut(self.active_tab)
                {
                    tab.scroll_to_relative(rel_y);
                }
            }
            Message::TerminalWheelScroll(raw_delta) => {
                if self.active_tab != SETTINGS_TAB_INDEX
                    && let Some(tab) = self.tabs.get_mut(self.active_tab)
                {
                    if tab.mouse_mode() {
                        self.scroll_accumulator += raw_delta;
                        let lines = self.scroll_accumulator as i32;
                        if lines != 0 {
                            self.scroll_accumulator -= lines as f32;
                            let button: u8 = if lines > 0 { 64 } else { 65 };
                            for _ in 0..lines.unsigned_abs() {
                                tab.send_mouse_event(button, 0, 0, true);
                            }
                        }
                    } else if tab.alt_screen() {
                        // Alt screen without mouse mode: convert scroll to arrow keys
                        self.scroll_accumulator += raw_delta;
                        let lines = self.scroll_accumulator as i32;
                        if lines != 0 {
                            self.scroll_accumulator -= lines as f32;
                            tab.send_scroll_as_arrows(lines);
                        }
                    } else {
                        self.scroll_accumulator = 0.0;
                        let delta = raw_delta.round() as i32;
                        if delta != 0 {
                            tab.scroll(delta);
                        }
                    }
                }
                if self
                    .tabs
                    .get(self.active_tab)
                    .is_some_and(|t| !t.mouse_mode() && !t.alt_screen())
                {
                    self.ignore_scrollable_sync = true;
                    return self.sync_terminal_scrollable_forced();
                }
            }
            Message::WindowResized(size) => {
                return self.handle_window_resized(size);
            }
            Message::AnimationTick => {
                let now = Instant::now();
                if !self.shell_picker_anim.is_animating(now) && !self.shell_picker_anim.value() {
                    self.show_shell_picker = false;
                    self.shell_picker_selected = 0;
                }
            }
            Message::ResizeDebounce => {
                if self.resize_debounce_seq != self.resize_debounce_spawned_seq {
                    // New resizes arrived during the wait -> restart timer
                    self.resize_debounce_spawned_seq = self.resize_debounce_seq;
                    return Task::perform(
                        async {
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        },
                        |()| Message::ResizeDebounce,
                    );
                }
                self.resize_debounce_pending = false;
                self.apply_resize();
            }

            // ── Window ──────────────────────────────────────────────
            Message::Exit => {
                return iced::exit();
            }
            Message::ApplyWindowStyle => {
                return self.handle_apply_window_style();
            }
            #[cfg(target_os = "windows")]
            Message::WindowMinimize => {
                return iced::window::latest().and_then(|id| iced::window::minimize(id, true));
            }
            #[cfg(target_os = "windows")]
            Message::WindowMaximize => {
                return iced::window::latest().and_then(iced::window::toggle_maximize);
            }
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            Message::WindowDrag => {
                return iced::window::latest().and_then(iced::window::drag);
            }
        }

        Task::none()
    }

    fn handle_key_pressed(
        &mut self,
        key: Key,
        modifiers: iced::keyboard::Modifiers,
        text: Option<String>,
    ) -> Task<Message> {
        if self.show_shell_picker {
            match key {
                Key::Named(Named::Escape) => {
                    return self.update(Message::CloseShellPicker);
                }
                Key::Named(Named::ArrowUp) => {
                    self.shift_shell_picker_selection(-1);
                }
                Key::Named(Named::ArrowDown) => {
                    self.shift_shell_picker_selection(1);
                }
                Key::Named(Named::Enter) => {
                    return self.confirm_shell_picker_selection();
                }
                _ => {}
            }
            return Task::none();
        }

        if let Some(task) = self.handle_app_shortcut(&key, modifiers) {
            return task;
        }

        if self.active_tab == SETTINGS_TAB_INDEX {
            return Task::none();
        }

        // Copy: Cmd+C (macOS) / Ctrl+Shift+C (other)
        if is_copy_shortcut(&key, modifiers)
            && let Some(tab) = self.tabs.get_mut(self.active_tab)
            && let Some(text) = tab.selected_text()
        {
            tab.clear_selection();
            return iced::clipboard::write(text);
        }
        // No selection → fall through to send Ctrl+C to terminal

        // Paste: Cmd+V (macOS) / Ctrl+Shift+V (other)
        if is_paste_shortcut(&key, modifiers) {
            return iced::clipboard::read()
                .map(|content| Message::PasteClipboard(content.unwrap_or_default()));
        }

        // Ignore modifier-only key presses
        if matches!(
            key,
            Key::Named(
                Named::Super
                    | Named::Control
                    | Named::Shift
                    | Named::Alt
                    | Named::Meta
                    | Named::Hyper
            )
        ) {
            return Task::none();
        }

        // Clear selection on actual key input
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.clear_selection();
            tab.handle_key(&key, modifiers, text.as_deref());
        }
        self.ignore_scrollable_sync = true;
        self.sync_terminal_scrollable()
    }

    fn handle_apply_window_style(&mut self) -> Task<Message> {
        if self.window_style_applied {
            return Task::none();
        }
        self.window_style_applied = true;

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let theme = self.config.theme.clone();
            iced::window::latest()
                .and_then(move |id| {
                    let theme = theme.clone();
                    iced::window::run(id, move |window| {
                        if let Ok(handle) = window.window_handle() {
                            crate::platform::apply_style(handle, &theme);
                        }
                    })
                })
                .discard()
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Task::none()
        }
    }
}

fn is_copy_shortcut(key: &Key, modifiers: iced::keyboard::Modifiers) -> bool {
    if let Key::Character(c) = key
        && c.eq_ignore_ascii_case("c")
    {
        #[cfg(target_os = "macos")]
        return modifiers.logo();
        #[cfg(not(target_os = "macos"))]
        return modifiers.control() && modifiers.shift();
    }
    false
}

fn is_paste_shortcut(key: &Key, modifiers: iced::keyboard::Modifiers) -> bool {
    if let Key::Character(c) = key
        && c.eq_ignore_ascii_case("v")
    {
        #[cfg(target_os = "macos")]
        return modifiers.logo();
        #[cfg(not(target_os = "macos"))]
        return modifiers.control() && modifiers.shift();
    }
    false
}
