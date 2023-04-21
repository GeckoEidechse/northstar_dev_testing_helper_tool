use core::time;

use self::util::{apply_launcher_pr, apply_mods_pr, find_game_install_path};
use self_update::cargo_crate_version;

mod util;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // Filter
    #[serde(skip)]
    filter_content: String,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    error_indicator: i32,

    #[serde(skip)]
    json_response: serde_json::Value,

    #[serde(skip)]
    scale_factor: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "/path/to/titanfall2".to_owned(),
            filter_content: "".to_owned(),
            error_indicator: 0,
            json_response: serde_json::Value::Null,
            scale_factor: -1.0,
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
            filter_content: filter_content_string,
            error_indicator: error_indicator_value,
            json_response,
            scale_factor,
        } = self;

        if *error_indicator_value != 0 {
            // Stupid way to get the error window to show for a bit
            // This should be replaced with a proper implementation later
            std::thread::sleep(time::Duration::from_millis(3000));
            *error_indicator_value = 0;
        }

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add(egui::Slider::new(scale_factor, 0.5..=5.0).text("Scaling factor"));
            // Stupide way to use default scale until value was updated
            if scale_factor > &mut 0.0 {
                ctx.set_pixels_per_point(*scale_factor);
            }
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
                ui.label("| ");
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Side Panel");

                ui.label("Titanfall2 install location:");
                ui.text_edit_singleline(game_install_path);
                if ui.button("Try detect install path").clicked() {
                    match find_game_install_path() {
                        Ok(found_install_path) => {
                            println!("Found install at {}", found_install_path);
                            *game_install_path = found_install_path;
                        }
                        Err(err) => {
                            println!("{}", err);
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("Error: {}", err))
                                        .color(egui::Color32::RED),
                                );
                            });

                            *error_indicator_value = 1;
                        }
                    }
                }

                ui.label(""); // simple spacer

                if ui.button("Refresh NorthstarMods PRs").clicked() {
                    match util::check_github_api(
                        "https://api.github.com/repos/R2Northstar/NorthstarMods/pulls",
                    ) {
                        Ok(result) => {
                            println!("Successful fetch");
                            *json_response = result;
                        }
                        Err(err) => {
                            println!("{}", err);
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("Error: {}", err))
                                        .color(egui::Color32::RED),
                                );
                            });

                            *error_indicator_value = 1;
                        }
                    }
                }

                ui.label(""); // simple spacer

                if ui.button("Refresh NorthstarLauncher PRs").clicked() {
                    match util::check_github_api(
                        "https://api.github.com/repos/R2Northstar/NorthstarLauncher/pulls",
                    ) {
                        Ok(result) => {
                            println!("Successful fetch");
                            *json_response = result;
                        }
                        Err(err) => {
                            println!("{}", err);
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("Error: {}", err))
                                        .color(egui::Color32::RED),
                                );
                            });

                            *error_indicator_value = 1;
                        }
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("powered by ");
                        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                        ui.label(" and ");
                        ui.hyperlink_to(
                            "eframe",
                            "https://github.com/emilk/egui/tree/master/eframe",
                        );
                    });
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

            // Deprecation notice
            ui.hyperlink_to(
                "Note that this tool has been deprecated in favour of integrating functionality into FlightCore!",
                "https://github.com/R2NorthstarTools/FlightCore/blob/ca65fb29fc2895e1912d931b4f486388fabaf7bd/docs/DEV-TOOLS.md#northstar",
            );
            // Filter field
            ui.label("Filter:");
            ui.text_edit_singleline(filter_content_string);

            egui::ScrollArea::vertical().show(ui, |ui| match json_response.as_array() {
                None => {
                    ui.label("No data, use refresh button on sidebar");
                }
                Some(json_response_array) => {
                    for elem in json_response_array {
                        let pr_number =
                            elem.get("number").and_then(|value| value.as_i64()).unwrap();
                        let pr_title = elem.get("title").and_then(|value| value.as_str()).unwrap();
                        let pr_url = elem.get("url").and_then(|value| value.as_str()).unwrap();

                        // Skip if not in filter
                        if !format!("{}: {}", pr_number, pr_title)
                            .to_lowercase()
                            .contains(&filter_content_string.to_string().to_lowercase())
                        {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Apply PR").clicked() {
                                println!("Attempting to install \"{}\"", pr_title);
                                println!("from: {}", pr_url);
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
                                        *error_indicator_value = 1;
                                    }
                                }
                            } else {
                                // This is a quick and dirty way to colour PR that don't have a testing labels as lighter colour to indicate no need for testing.
                                // In the future this should be rewritten more nicely and maybe allow filtering by label
                                // Also the hardcoded value should be a constant at the top of the source file.
                                let labels = elem
                                    .get("labels")
                                    .and_then(|value| value.as_array())
                                    .unwrap();
                                let mut temp_bool = false;
                                for elem in labels {
                                    let label_name =
                                        elem.get("name").and_then(|value| value.as_str()).unwrap();
                                    if label_name == "needs testing" {
                                        temp_bool = true;
                                    }
                                    // dbg!(label_name);
                                }
                                if temp_bool {
                                    ui.label(
                                        egui::RichText::new(format!("{}: {}", pr_number, pr_title))
                                            .strong(),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!("{}: {}", pr_number, pr_title))
                                            .color(egui::Color32::GRAY),
                                    );
                                }
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
