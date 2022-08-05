use core::time;

use self::util::{apply_launcher_pr, apply_mods_pr};
use self_update::cargo_crate_version;

mod util;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: i32,

    #[serde(skip)]
    json_response: serde_json::Value,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "/path/to/titanfall2".to_owned(),
            value: 0,
            json_response: serde_json::Value::Null,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self {
            label: game_install_path,
            value,
            json_response,
        } = self;

        if *value != 0 {
            // Stupid way to get the error window to show for a bit
            // This should be replaced with a proper implementation later
            std::thread::sleep(time::Duration::from_millis(3000));
            *value = 0;
        }

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.label("Titanfall2 install location:");
            ui.text_edit_singleline(game_install_path);

            ui.label(""); // simple spacer

            if ui.button("Refresh NorthstarMods PRs").clicked() {
                *json_response = util::check_github_api(
                    "https://api.github.com/repos/R2Northstar/NorthstarMods/pulls",
                )
                .expect("Failed request");
            }

            ui.label(""); // simple spacer

            if ui.button("Refresh NorthstarLauncher PRs").clicked() {
                *json_response = util::check_github_api(
                    "https://api.github.com/repos/R2Northstar/NorthstarLauncher/pulls",
                )
                .expect("Failed request");
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/eframe");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading(format!(
                "Northstar dev testing helper tool (v{})",
                cargo_crate_version!()
            ));
            ui.hyperlink_to(
                "Source Code",
                "https://github.com/GeckoEidechse/northstar_dev_testing_helper_tool",
            );
            egui::warn_if_debug_build(ui);
            egui::ScrollArea::vertical().show(ui, |ui| match json_response.as_array() {
                None => {
                    ui.label("No data, use refresh button on sidebar");
                }
                Some(json_response_array) => {
                    for elem in json_response_array {
                        let mut pr_number = 0;
                        let mut pr_title = "";
                        let mut pr_url = "";
                        for val in elem.as_object().unwrap() {
                            let (key, v) = val;

                            if key == "number" {
                                pr_number = v.as_i64().unwrap();
                            }
                            if key == "title" {
                                pr_title = v.as_str().unwrap();
                            }
                            if key == "url" {
                                pr_url = v.as_str().unwrap();
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Apply PR").clicked() {
                                let apply_pr = if pr_url.contains("NorthstarLauncher") {
                                    apply_launcher_pr
                                } else {
                                    apply_mods_pr
                                };
                                let apply_pr_result =
                                    apply_pr(pr_number, game_install_path, json_response.clone());
                                match apply_pr_result {
                                    Ok(_) => println!("All good?"),
                                    Err(err) => {
                                        println!("{}", err);
                                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("Error: {}", err))
                                                    .color(egui::Color32::RED),
                                            );
                                        });
                                        *value = 1;
                                    }
                                }
                            } else {
                                ui.label(format!("{}: {}", pr_number, pr_title));
                            }
                        });
                    }
                }
            });
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}
