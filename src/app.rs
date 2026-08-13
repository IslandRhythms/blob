use std::collections::HashSet;
use std::sync::mpsc;

use eframe::egui::{self, Align, Color32, Key, Layout, RichText, TextEdit};
use zeroize::Zeroize;

use crate::vault::{self, Entry, UnlockedVault};

pub const ICON_PNG: &[u8] = include_bytes!("../icon.png");

pub struct VaultApp {
    screen: Screen,
    icon: egui::TextureHandle,
}

enum Screen {
    Locked(LockedState),
    Unlocked(UnlockedState),
}

#[derive(Default)]
struct LockedState {
    password: String,
    confirm: String,
    error: Option<String>,
    is_new: bool,
    confirm_reset: bool,
    pending: Option<mpsc::Receiver<anyhow::Result<UnlockedVault>>>,
}

struct UnlockedState {
    vault: UnlockedVault,
    revealed: HashSet<usize>,
    editing: Option<EditState>,
    search: String,
    new_name: String,
    new_username: String,
    new_password: String,
    show_new_password: bool,
    status: Option<String>,
}

struct EditState {
    index: usize,
    name: String,
    username: String,
    password: String,
    show_password: bool,
}

impl EditState {
    fn from_entry(index: usize, entry: &Entry) -> Self {
        Self {
            index,
            name: entry.name.clone(),
            username: entry.username.clone(),
            password: entry.password.clone(),
            show_password: false,
        }
    }
}

impl Drop for EditState {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

impl UnlockedState {
    fn new(vault: UnlockedVault) -> Self {
        Self {
            vault,
            revealed: HashSet::new(),
            editing: None,
            search: String::new(),
            new_name: String::new(),
            new_username: String::new(),
            new_password: String::new(),
            show_new_password: false,
            status: None,
        }
    }
}

impl VaultApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            screen: Screen::Locked(LockedState {
                is_new: !vault::exists(),
                ..Default::default()
            }),
            icon: load_app_icon(&cc.egui_ctx),
        }
    }
}

fn load_app_icon(ctx: &egui::Context) -> egui::TextureHandle {
    let image = image::load_from_memory(ICON_PNG)
        .expect("app icon")
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3);
    let rgba = image.into_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    ctx.load_texture("app-icon", color_image, Default::default())
}

fn brand_heading(ui: &mut egui::Ui, icon: &egui::TextureHandle, size: f32) {
    ui.horizontal(|ui| {
        ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(size, size)));
        ui.heading(RichText::new("BlobVault").size(size));
    });
}

impl eframe::App for VaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let next = match &mut self.screen {
            Screen::Locked(state) => locked_ui(ctx, state, &self.icon),
            Screen::Unlocked(state) => unlocked_ui(ctx, state, &self.icon),
        };
        if let Some(next) = next {
            self.screen = next;
            ctx.request_repaint();
        }
    }
}

fn locked_ui(
    ctx: &egui::Context,
    state: &mut LockedState,
    icon: &egui::TextureHandle,
) -> Option<Screen> {
    // Poll the background unlock thread, if one is running.
    let mut unlocked: Option<UnlockedVault> = None;
    if let Some(rx) = &state.pending {
        match rx.try_recv() {
            Ok(Ok(vault)) => {
                state.pending = None;
                unlocked = Some(vault);
            }
            Ok(Err(e)) => {
                state.pending = None;
                state.error = Some(format!("{e}"));
            }
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.pending = None;
                state.error = Some("unlock failed unexpectedly".into());
            }
        }
    }
    let busy = state.pending.is_some();

    let mut submit = false;
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            brand_heading(ui, icon, 28.0);
            ui.add_space(4.0);
            ui.label(if state.is_new {
                "Create a master password to set up your vault."
            } else {
                "Enter your master password to unlock."
            });
            ui.add_space(16.0);

            ui.add_enabled_ui(!busy, |ui| {
                let pw = ui.add(
                    TextEdit::singleline(&mut state.password)
                        .password(true)
                        .hint_text("Master password")
                        .desired_width(240.0),
                );
                if !busy && ctx.memory(|m| m.focused().is_none()) {
                    pw.request_focus();
                }
                if pw.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    submit = true;
                }

                if state.is_new {
                    ui.add_space(6.0);
                    let confirm = ui.add(
                        TextEdit::singleline(&mut state.confirm)
                            .password(true)
                            .hint_text("Confirm password")
                            .desired_width(240.0),
                    );
                    if confirm.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        submit = true;
                    }
                }

                ui.add_space(10.0);
                let label = if state.is_new { "Create vault" } else { "Unlock" };
                if ui.button(label).clicked() {
                    submit = true;
                }
            });

            if busy {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.spinner();
                        ui.label(RichText::new("Unlocking…").weak());
                    });
                });
            }

            if let Some(err) = &state.error {
                ui.add_space(8.0);
                ui.colored_label(Color32::from_rgb(220, 80, 80), err);
            }

            if !state.is_new && !busy {
                ui.add_space(28.0);
                if state.confirm_reset {
                    ui.scope(|ui| {
                        ui.set_max_width(300.0);
                        ui.colored_label(
                            Color32::from_rgb(220, 80, 80),
                            "Resetting permanently deletes the vault and every \
                             saved account. There is no way to recover them.",
                        );
                    });
                    ui.add_space(8.0);
                    let delete_button = egui::Button::new(
                        RichText::new("Delete vault and start fresh").color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(170, 40, 40));
                    if ui.add(delete_button).clicked() {
                        match vault::reset() {
                            Ok(()) => {
                                state.is_new = true;
                                state.confirm_reset = false;
                                state.error = None;
                                state.password.zeroize();
                                state.password.clear();
                                state.confirm.zeroize();
                                state.confirm.clear();
                            }
                            Err(e) => {
                                state.confirm_reset = false;
                                state.error = Some(format!("{e}"));
                            }
                        }
                    }
                    ui.add_space(4.0);
                    if ui.button("Cancel").clicked() {
                        state.confirm_reset = false;
                    }
                } else if ui
                    .small_button(RichText::new("Forgot password? Reset vault…").weak())
                    .clicked()
                {
                    state.confirm_reset = true;
                }
            }
        });
    });

    if submit && !busy {
        start_unlock(state);
    }
    unlocked.map(|vault| Screen::Unlocked(UnlockedState::new(vault)))
}

/// Validate the form, then run the (slow) Argon2 derivation on a background
/// thread so the UI keeps painting.
fn start_unlock(state: &mut LockedState) {
    if state.password.is_empty() {
        state.error = Some("Password cannot be empty.".into());
        return;
    }
    if state.is_new && state.password != state.confirm {
        state.error = Some("Passwords do not match.".into());
        return;
    }
    state.error = None;

    let mut password = std::mem::take(&mut state.password);
    state.confirm.zeroize();
    state.confirm.clear();

    let is_new = state.is_new;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = if is_new {
            vault::create(&password)
        } else {
            vault::unlock(&password)
        };
        password.zeroize();
        let _ = tx.send(result);
    });
    state.pending = Some(rx);
}

fn unlocked_ui(
    ctx: &egui::Context,
    state: &mut UnlockedState,
    icon: &egui::TextureHandle,
) -> Option<Screen> {
    let mut lock = false;
    let mut dirty = false;
    let mut to_delete: Option<usize> = None;
    let mut start_edit: Option<usize> = None;
    let mut save_edit = false;
    let mut cancel_edit = false;

    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            brand_heading(ui, icon, 18.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Lock").clicked() {
                    lock = true;
                }
                ui.add(
                    TextEdit::singleline(&mut state.search)
                        .hint_text("🔍 Search…")
                        .desired_width(180.0),
                );
            });
        });
        ui.add_space(6.0);
    });

    egui::TopBottomPanel::bottom("add_panel").show(ctx, |ui| {
        ui.add_space(4.0);
        if let Some(status) = &state.status {
            ui.label(RichText::new(status).weak());
        }
        egui::CollapsingHeader::new("➕ Add account").show(ui, |ui| {
            egui::Grid::new("add_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut state.new_name);
                    ui.end_row();

                    ui.label("Username");
                    ui.text_edit_singleline(&mut state.new_username);
                    ui.end_row();

                    ui.label("Password");
                    ui.horizontal(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut state.new_password)
                                .password(!state.show_new_password)
                                .desired_width(180.0),
                        );
                        if ui
                            .selectable_label(state.show_new_password, "👁")
                            .on_hover_text("Show password")
                            .clicked()
                        {
                            state.show_new_password = !state.show_new_password;
                        }
                        if ui.button("Generate").clicked() {
                            state.new_password = vault::generate_password(20);
                            state.show_new_password = true;
                        }
                    });
                    ui.end_row();
                });
            ui.add_space(4.0);
            if ui.button("Add").clicked() {
                if state.new_name.trim().is_empty() {
                    state.status = Some("Name is required.".into());
                } else {
                    state.vault.vault.entries.push(Entry {
                        name: state.new_name.trim().to_owned(),
                        username: state.new_username.trim().to_owned(),
                        password: state.new_password.clone(),
                    });
                    state.new_name.clear();
                    state.new_username.clear();
                    state.new_password.zeroize();
                    state.new_password.clear();
                    state.show_new_password = false;
                    state.status = Some("Account added.".into());
                    dirty = true;
                }
            }
        });
        ui.add_space(6.0);
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let filter = state.search.to_lowercase();
                let entries = &state.vault.vault.entries;

                if entries.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Image::new(icon).fit_to_exact_size(egui::vec2(96.0, 96.0)),
                        );
                        ui.add_space(12.0);
                        ui.label(RichText::new("No accounts yet.").weak());
                        ui.label(RichText::new("Add one below to get started.").weak());
                    });
                    return;
                }

                for (i, entry) in entries.iter().enumerate() {
                    let editing_this = state.editing.as_ref().is_some_and(|e| e.index == i);
                    if !editing_this
                        && !filter.is_empty()
                        && !entry.name.to_lowercase().contains(&filter)
                        && !entry.username.to_lowercase().contains(&filter)
                    {
                        continue;
                    }

                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        if editing_this {
                            let edit = state.editing.as_mut().unwrap();
                            egui::Grid::new(("edit_grid", i))
                                .num_columns(2)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Name");
                                    ui.text_edit_singleline(&mut edit.name);
                                    ui.end_row();

                                    ui.label("Username");
                                    ui.text_edit_singleline(&mut edit.username);
                                    ui.end_row();

                                    ui.label("Password");
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            TextEdit::singleline(&mut edit.password)
                                                .password(!edit.show_password)
                                                .desired_width(180.0),
                                        );
                                        if ui
                                            .selectable_label(edit.show_password, "👁")
                                            .on_hover_text("Show password")
                                            .clicked()
                                        {
                                            edit.show_password = !edit.show_password;
                                        }
                                        if ui.button("Generate").clicked() {
                                            edit.password = vault::generate_password(20);
                                            edit.show_password = true;
                                        }
                                    });
                                    ui.end_row();
                                });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    save_edit = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    cancel_edit = true;
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&entry.name).strong().size(16.0));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("🗑").on_hover_text("Delete account").clicked() {
                                        to_delete = Some(i);
                                    }
                                    if ui.button("Edit").clicked() {
                                        start_edit = Some(i);
                                    }
                                });
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("User").weak());
                                ui.label(&entry.username);
                                if ui
                                    .small_button("📋")
                                    .on_hover_text("Copy username")
                                    .clicked()
                                {
                                    ctx.copy_text(entry.username.clone());
                                    state.status =
                                        Some(format!("Copied username for {}.", entry.name));
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Pass").weak());
                                let revealed = state.revealed.contains(&i);
                                if revealed {
                                    ui.label(RichText::new(&entry.password).monospace());
                                } else {
                                    ui.label(RichText::new("••••••••••••").monospace());
                                }
                                if ui
                                    .selectable_label(revealed, "👁")
                                    .on_hover_text(if revealed { "Hide" } else { "Reveal" })
                                    .clicked()
                                {
                                    if revealed {
                                        state.revealed.remove(&i);
                                    } else {
                                        state.revealed.insert(i);
                                    }
                                }
                                if ui
                                    .small_button("📋")
                                    .on_hover_text("Copy password")
                                    .clicked()
                                {
                                    ctx.copy_text(entry.password.clone());
                                    state.status =
                                        Some(format!("Copied password for {}.", entry.name));
                                }
                            });
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    });

    if let Some(i) = start_edit {
        state.editing = Some(EditState::from_entry(i, &state.vault.vault.entries[i]));
    }
    if cancel_edit {
        state.editing = None;
    }
    if save_edit {
        if let Some(edit) = &state.editing {
            if edit.name.trim().is_empty() {
                state.status = Some("Name is required.".into());
            } else {
                let entry = &mut state.vault.vault.entries[edit.index];
                entry.name = edit.name.trim().to_owned();
                entry.username = edit.username.trim().to_owned();
                entry.password = edit.password.clone();
                state.status = Some(format!("Saved changes to {}.", entry.name));
                state.editing = None;
                dirty = true;
            }
        }
    }

    if let Some(i) = to_delete {
        state.vault.vault.entries.remove(i);
        // Indices shifted; drop reveal state rather than tracking the shuffle.
        state.revealed.clear();
        match &mut state.editing {
            Some(edit) if edit.index == i => state.editing = None,
            Some(edit) if edit.index > i => edit.index -= 1,
            _ => {}
        }
        dirty = true;
        state.status = Some("Account deleted.".into());
    }

    if dirty {
        if let Err(e) = state.vault.save() {
            state.status = Some(format!("Failed to save vault: {e}"));
        }
    }

    if lock {
        return Some(Screen::Locked(LockedState {
            is_new: false,
            ..Default::default()
        }));
    }
    None
}
