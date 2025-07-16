// Copyright (C) 2025 Dxian998
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![windows_subsystem = "windows"]
use eframe::egui;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 447.0])
            .with_min_inner_size([520.0, 447.0])
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Stellar Blade SteamID Replacer",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SteamIDReplacerApp::default()))
        }),
    )
}

fn load_icon() -> egui::IconData {
    use ico::IconDir;
    let ico_bytes = include_bytes!("icon.ico");
    let icon_dir = IconDir::read(std::io::Cursor::new(ico_bytes)).expect("Invalid .ico data");
    let entry = &icon_dir.entries()[0];
    let image = entry.decode().expect("Failed to decode ICO image");
    let (width, height) = (image.width(), image.height());
    let pixels = image.rgba_data().to_vec();
    egui::IconData {
        rgba: pixels,
        width,
        height,
    }
}

#[derive(Default)]
struct SteamIDReplacerApp {
    file_path: String,
    new_steamid: String,
    current_steamid: String,
    status: String,
    backup_filename: String,
    show_about: bool,
    show_help: bool,
    drag_hover: bool,
    icon_texture: Option<egui::TextureHandle>,
    transfer_demo_save: bool,
    show_demo_error: bool,
}

impl eframe::App for SteamIDReplacerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                if self.show_about {
                    self.show_about = false;
                } else if self.show_help {
                    self.show_help = false;
                }
            }
        });

        if self.icon_texture.is_none() {
            let icon_data = load_icon();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [icon_data.width as usize, icon_data.height as usize],
                &icon_data.rgba,
            );
            self.icon_texture = Some(ctx.load_texture("icon", image, Default::default()));
        }

        ctx.input(|i| {
            if !i.raw.hovered_files.is_empty() {
                self.drag_hover = true;
            } else {
                self.drag_hover = false;
            }
            if !i.raw.dropped_files.is_empty() {
                if let Some(file) = i.raw.dropped_files.first() {
                    if let Some(path) = &file.path {
                        self.file_path = path.to_string_lossy().to_string();
                        self.status = "File loaded via drag & drop".to_string();
                        self.backup_filename.clear();
                        self.read_current_steamid();
                    }
                }
            }
        });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("🗁 Open File").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Save Files", &["sav"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            self.file_path = path.to_string_lossy().to_string();
                            self.status = "File has been selected. Ready!".to_string();
                            self.backup_filename.clear();
                            self.read_current_steamid();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("❓ Help").clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    if ui.button("ℹ About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 140.0);
                    if let Some(tex) = &self.icon_texture {
                        ui.image((tex.id(), egui::vec2(24.0, 24.0)));
                        ui.add_space(8.0);
                    }
                    ui.heading("Stellar Blade SteamID Replacer");
                });
                ui.add_space(8.0);
                ui.label("Replace SteamID in Stellar Blade save files");
                ui.add_space(20.0);
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("📁 File Selection");
                    ui.horizontal(|ui| {
                        ui.label("File Path:");
                        let text_width = ui.available_width() - 80.0;
                        let response = ui.add_sized(
                            [text_width, 20.0],
                            egui::TextEdit::singleline(&mut self.file_path)
                                .hint_text("Drag & drop a file or click Browse..."),
                        );
                        if response.changed() {
                            self.status.clear();
                            self.backup_filename.clear();
                            self.read_current_steamid();
                        }
                        if ui.button("📂 Browse").clicked() {
                            #[cfg(target_os = "windows")]
                            {
                                if let Ok(userprofile) = std::env::var("USERPROFILE") {
                                    let path = PathBuf::from(userprofile)
                                        .join("AppData")
                                        .join("Local")
                                        .join("SB")
                                        .join("Saved")
                                        .join("SaveGames");
                                    if path.exists() {
                                        if let Some(file_path) = rfd::FileDialog::new()
                                            .set_directory(&path)
                                            .add_filter("Save Files", &["sav"])
                                            .add_filter("All Files", &["*"])
                                            .pick_file()
                                        {
                                            self.file_path = file_path.to_string_lossy().to_string();
                                            self.status = "File has been selected. Ready!".to_string();
                                            self.backup_filename.clear();
                                            self.read_current_steamid();
                                        }
                                    } else {
                                        if let Some(file_path) = rfd::FileDialog::new()
                                            .add_filter("Save Files", &["sav"])
                                            .add_filter("All Files", &["*"])
                                            .pick_file()
                                        {
                                            self.file_path = file_path.to_string_lossy().to_string();
                                            self.status = "File has been selected. Ready!".to_string();
                                            self.backup_filename.clear();
                                            self.read_current_steamid();
                                        }
                                    }
                                }
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                if let Some(file_path) = rfd::FileDialog::new()
                                    .add_filter("Save Files", &["sav"])
                                    .add_filter("All Files", &["*"])
                                    .pick_file()
                                {
                                    self.file_path = file_path.to_string_lossy().to_string();
                                    self.status = "File has been selected. Ready!".to_string();
                                    self.backup_filename.clear();
                                    self.read_current_steamid();
                                }
                            }
                        }
                    });
                    if self.drag_hover {
                        ui.colored_label(egui::Color32::LIGHT_GRAY, "📤 Drop your file here");
                    }
                    ui.horizontal(|ui| {
                        ui.label("Current SteamID:");
                        if self.current_steamid.is_empty() {
                            ui.colored_label(egui::Color32::LIGHT_GRAY, "No valid file selected");
                        } else {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, &self.current_steamid);
                        }
                    });
                });
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("🔢 New SteamID");
                    ui.horizontal(|ui| {
                        ui.label("SteamID:");
                        let text_width = ui.available_width();
                        let response = ui.add_sized(
                            [text_width, 20.0],
                            egui::TextEdit::singleline(&mut self.new_steamid)
                                .hint_text("Enter 17-digit SteamID (e.g., 76561198123456789)"),
                        );
                        if response.changed() {
                            self.status.clear();
                            self.backup_filename.clear();
                        }
                    });
                    ui.label("💡 SteamID must be 17 digits starting with '7656'");
                });
            });
            
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    let transfer_response = ui.checkbox(&mut self.transfer_demo_save, "📋 Transfer Demo Save (removes 'Demo00' from filename)");
                    if transfer_response.changed() && self.transfer_demo_save {
                        if !self.file_path.contains("Demo00") {
                            self.status = "❌ Error: File does not contain 'Demo00'. Cannot transfer demo save.".to_string();
                            self.show_demo_error = true;
                        } else {
                            self.show_demo_error = false;
                        }
                    }
                });
            });

            ui.add_space(12.0);

            ui.vertical_centered(|ui| {
                let button_enabled = !self.file_path.is_empty()
                    && !self.new_steamid.is_empty()
                    && (!self.transfer_demo_save || self.file_path.contains("Demo00"));
                let button = egui::Button::new("🔄 Replace SteamID")
                    .min_size(egui::vec2(160.0, 32.0));
                let response = ui.add_enabled(button_enabled, button);
                if response.clicked() {
                    if self.file_path.is_empty() {
                        self.status = "⚠️ Please select a file first".to_string();
                        self.backup_filename.clear();
                    } else if self.new_steamid.is_empty() {
                        self.status = "⚠️ Please enter a new SteamID".to_string();
                        self.backup_filename.clear();
                    } else if !self.current_steamid.is_empty() && self.current_steamid == self.new_steamid {
                        self.status = "⚠️ Error: New SteamID is the same as current SteamID".to_string();
                        self.backup_filename.clear();
                    } else if self.transfer_demo_save && !self.file_path.contains("Demo00") {
                        self.status = "❌ Error: File does not contain 'Demo00'. Cannot transfer demo save.".to_string();
                        self.backup_filename.clear();
                    } else {
                        match self.replace_steamid() {
                            Ok((count, backup_name)) => {
                                if count > 0 {
                                    self.status = format!("✅ Successfully replaced SteamID!");
                                    self.backup_filename = backup_name;
                                    self.current_steamid = self.new_steamid.clone();
                                } else {
                                    self.status = "⚠️ No valid SteamIDs found in file".to_string();
                                    self.backup_filename.clear();
                                }
                            }
                            Err(e) => {
                                self.status = format!("❌ Error: {}", e);
                                self.backup_filename.clear();
                            }
                        }
                    }
                }
            });

            ui.add_space(12.0);
            ui.group(|ui| {
                ui.set_min_height(60.0);
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.heading("🍕 Status");
                    if self.status.is_empty() {
                        if !self.backup_filename.is_empty() {
                            ui.colored_label(
                                egui::Color32::LIGHT_GRAY,
                                format!("Backup saved as: {}", self.backup_filename),
                            );
                        }
                    } else {
                        if self.status.contains("Error") || self.status.contains("❌") {
                            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), &self.status);
                        } else if self.status.contains("Successfully") || self.status.contains("✅") {
                            ui.colored_label(egui::Color32::from_rgb(80, 200, 120), &self.status);
                        } else if self.status.contains("⚠️") {
                            ui.colored_label(egui::Color32::from_rgb(255, 165, 0), &self.status);
                        } else {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status);
                        }
                        if !self.backup_filename.is_empty() {
                            ui.colored_label(
                                egui::Color32::LIGHT_GRAY,
                                format!("Backup saved as: {}", self.backup_filename),
                            );
                        }
                    }
                });
            });

            ui.add_space(10.0);
        });

        self.show_dialogs(ctx);
    }
}

impl SteamIDReplacerApp {
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_about {
            egui::Window::new("About")
                .collapsible(false)
                .resizable(false)
                .default_width(370.0)
                .max_width(400.0)
                .max_height(320.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.heading("Stellar Blade SteamID Replacer");
                        ui.add_space(10.0);
                        ui.label("Version 1.0");
                        ui.add_space(5.0);
                        ui.label("A tool for replacing SteamID of Stellar Blade save files");
                        ui.label("Made by Dxian998 on NexusMods");
                        ui.add_space(15.0);
                        
                        ui.separator();
                        ui.add_space(10.0);
                        
                        ui.label("Features:");
                        ui.label("• Replace SteamID in Stellar Blade saves");
                        ui.label("• Show current SteamID in file");
                        ui.label("• Demo save transfer support");
                        ui.label("• Automatic backup creation");
                        ui.label("• Drag & drop support");
                        ui.label("• Safe file operations");
                        
                        ui.add_space(15.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("✅ Close").clicked() {
                                self.show_about = false;
                            }
                        });
                    });
                });
        }

        if self.show_help {
            egui::Window::new("Help")
                .collapsible(false)
                .resizable(false)
                .default_width(480.0)
                .max_width(520.0)
                .max_height(450.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading("How to use Stellar Blade SteamID Replacer");
                                });
                                ui.add_space(10.0);

                                ui.label("📋 Step-by-step guide:");
                                ui.add_space(5.0);

                                ui.label("1. 📁 Select your Stellar Blade save file:");
                                ui.label("   • Click 'Browse' button, or");
                                ui.label("   • Drag & drop the file into the window");
                                ui.add_space(5.0);

                                ui.label("2. 🔍 View current SteamID (if found):");
                                ui.label("   • The app will show the existing SteamID in the file");
                                ui.add_space(5.0);

                                ui.label("3. 🔢 Enter the new SteamID:");
                                ui.label("   • Must be exactly 17 digits");
                                ui.label("   • Must start with '7656'");
                                ui.label("   • Example: 76561198123456789");
                                ui.add_space(5.0);

                                ui.label("4. 📋 Transfer Demo Save (Optional):");
                                ui.label("   • Check this for demo save files");
                                ui.label("   • Removes 'Demo00' from filename");
                                ui.label("   • Keeps backup as demo file");
                                ui.add_space(5.0);

                                ui.label("5. 🔄 Click 'Replace SteamID'");
                                ui.label("   • A backup (.bak) will be created automatically");
                                ui.add_space(10.0);

                                ui.separator();
                                ui.label("🔍 Finding your SteamID:");
                                ui.label("• Visit steamid.io or steamidfinder.com");
                                ui.label("• Enter your Steam profile URL");
                                ui.label("• Copy the 'SteamID64' number");

                                ui.add_space(5.0);

                                ui.separator();
                                ui.colored_label(egui::Color32::from_rgb(80, 200, 120), "✅ Safety:");
                                ui.label("• Automatic backup creation (.bak extension)");
                                ui.label("• Original file is preserved before changes");
                                ui.label("• Prevents replacing with same SteamID");
                                ui.add_space(5.0);
                            });

                        ui.add_space(10.0);
                        ui.horizontal_centered(|ui| {
                            if ui.button("✅ Close").clicked() {
                                self.show_help = false;
                            }
                        });
                    });
                });
        }
    }
    
    fn read_current_steamid(&mut self) {
        self.current_steamid.clear();
        
        if self.file_path.is_empty() {
            return;
        }
        
        if let Ok(mut file) = File::open(&self.file_path) {
            let mut data = Vec::new();
            if file.read_to_end(&mut data).is_ok() {
                for i in 0..=data.len().saturating_sub(17) {
                    if self.is_steamid_at_position(&data, i) {
                        if let Ok(steamid) = String::from_utf8(data[i..i+17].to_vec()) {
                            self.current_steamid = steamid;
                            break;
                        }
                    }
                }
            }
        }
    }
    
    fn create_backup(&self) -> Result<String, String> {
        let backup_path = format!("{}.bak", self.file_path);
        
        std::fs::copy(&self.file_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
            
        Ok(backup_path)
    }

    fn replace_steamid(&self) -> Result<(usize, String), String> {
        if !self.is_valid_steamid(&self.new_steamid) {
            return Err("Invalid SteamID format. Must be 17 digits starting with 7656".to_string());
        }

        let backup_path = self.create_backup()?;
        
        let backup_filename = std::path::Path::new(&backup_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("backup.bak"))
            .to_string_lossy()
            .to_string();

        let mut file = File::open(&self.file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let mut replacements = 0;
        let mut i = 0;

        while i <= data.len().saturating_sub(17) {
            if self.is_steamid_at_position(&data, i) {
                for (j, &byte) in self.new_steamid.as_bytes().iter().enumerate() {
                    data[i + j] = byte;
                }
                replacements += 1;
                i += 17; 
            } else {
                i += 1;
            }
        }

        if replacements > 0 {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.file_path)
                .map_err(|e| {
                    let _ = std::fs::copy(&backup_path, &self.file_path);
                    format!("Failed to write to file: {}", e)
                })?;

            file.write_all(&data)
                .map_err(|e| {
                    let _ = std::fs::copy(&backup_path, &self.file_path);
                    format!("Failed to write to file: {}", e)
                })?;

            if self.transfer_demo_save {
                self.handle_demo_transfer()?;
            }
        }

        Ok((replacements, backup_filename))
    }

    fn handle_demo_transfer(&self) -> Result<(), String> {
        let path = std::path::Path::new(&self.file_path);
        
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy();
            
            if filename_str.contains("Demo00") {
                let new_filename = filename_str.replace("Demo00", "");
                
                if let Some(parent) = path.parent() {
                    let new_path = parent.join(&new_filename);
                    
                    std::fs::rename(&self.file_path, &new_path)
                        .map_err(|e| format!("Failed to transfer demo save: {}", e))?;
                }
            }
        }
        
        Ok(())
    }

    fn is_valid_steamid(&self, steamid: &str) -> bool {
        steamid.len() == 17 
            && steamid.starts_with("7656") 
            && steamid.chars().all(|c| c.is_ascii_digit())
    }

    fn is_steamid_at_position(&self, data: &[u8], pos: usize) -> bool {
        if pos + 17 > data.len() {
            return false;
        }

        if &data[pos..pos + 4] != b"7656" {
            return false;
        }

        for i in pos..pos + 17 {
            if !data[i].is_ascii_digit() {
                return false;
            }
        }

        true
    }
}