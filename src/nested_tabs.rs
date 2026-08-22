// Reusable nested-tab layout helpers.
//
// This module owns the vertical nested-tab navigation used by the Control
// Center Configuration tab and the compact configuration-page presentation.
// Existing configuration persistence remains owned by editor_layout.rs /
// edit_shader.rs / manage_configuration.rs.

use std::sync::OnceLock;

use crate::editor_layout::{
    ControlConfiguration,
    PolicyDisplayRow,
    PolicyTarget,
};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigurationNestedTab {
    Appearance,
    Screensaver,
    Wallpaper,
    Rendering,
}


impl ConfigurationNestedTab {
    const ALL: [
        ConfigurationNestedTab;
        4
    ] = [
        ConfigurationNestedTab::Appearance,
        ConfigurationNestedTab::Screensaver,
        ConfigurationNestedTab::Wallpaper,
        ConfigurationNestedTab::Rendering,
    ];


    fn label(
        self,
    ) -> &'static str {
        match self {
            ConfigurationNestedTab::Appearance => "Appearance",
            ConfigurationNestedTab::Screensaver => "Screensaver",
            ConfigurationNestedTab::Wallpaper => "Wallpaper",
            ConfigurationNestedTab::Rendering => "Rendering",
        }
    }
}


pub fn draw_configuration(
    ui: &mut egui::Ui,
    configuration: &mut Option<ControlConfiguration>,
    baseline: Option<&ControlConfiguration>,
    policy_rows: &[PolicyDisplayRow],
    save_requested: &mut bool,
    status_message: &mut String,
) {
    let Some(configuration) =
        configuration.as_mut()
    else {
        ui.label(
            egui::RichText::new(
                "Configuration is not available."
            )
            .weak(),
        );
        return;
    };


    let selected_id =
        egui::Id::new(
            "screenshaver_configuration_nested_tab"
        );


    let mut selected =
        ui.ctx()
            .data(
                |data| {
                    data.get_temp::<ConfigurationNestedTab>(
                        selected_id
                    )
                    .unwrap_or(
                        ConfigurationNestedTab::Appearance
                    )
                }
            );


    const FOOTER_HEIGHT: f32 =
        40.0;

    let full_width =
        ui.available_width();

    let full_height =
        ui.available_height();

    let content_height =
        (
            full_height
                - FOOTER_HEIGHT
        )
        .max(
            120.0
        );


    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            content_height,
        ),
        egui::Layout::top_down(
            egui::Align::Min
        ),
        |ui| {
            ui.horizontal(
                |ui| {
                    draw_nested_tab_rail(
                        ui,
                        &mut selected,
                    );


                    ui.separator();


                    ui.add_space(
                        12.0
                    );


                    ui.vertical(
                        |ui| {
                            ui.set_width(
                                ui.available_width()
                            );

                            match selected {
                                ConfigurationNestedTab::Appearance => {
                                    draw_appearance(
                                        ui,
                                        configuration,
                                    );
                                }

                                ConfigurationNestedTab::Screensaver => {
                                    draw_target_page(
                                        ui,
                                        configuration,
                                        policy_rows,
                                        PolicyTarget::Screensaver,
                                        status_message,
                                    );
                                }

                                ConfigurationNestedTab::Wallpaper => {
                                    draw_target_page(
                                        ui,
                                        configuration,
                                        policy_rows,
                                        PolicyTarget::Wallpaper,
                                        status_message,
                                    );
                                }

                                ConfigurationNestedTab::Rendering => {
                                    draw_rendering_placeholders(
                                        ui,
                                        configuration,
                                    );
                                }
                            }
                        },
                    );
                },
            );
        },
    );


    let dirty =
        baseline
            .map(
                |baseline| {
                    &*configuration
                        != baseline
                }
            )
            .unwrap_or(
                false
            );

    let single_policy_missing =
        (
            configuration.screensaver_display
                == "single"
                && configuration
                    .screensaver_single_policy_id
                    .is_none()
        )
        || (
            configuration.wallpaper_display
                == "single"
                && configuration
                    .wallpaper_single_policy_id
                    .is_none()
        );


    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            FOOTER_HEIGHT,
        ),
        egui::Layout::left_to_right(
            egui::Align::Center
        ),
        |ui| {
            ui.add_space(
                8.0
            );

            let save_response =
                ui.add_enabled(
                    dirty
                        && !single_policy_missing,
                    egui::Button::new(
                        "Save Configuration"
                    ),
                );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    "Saving configuration..."
                        .to_string();
            }


            ui.add_space(
                8.0
            );


            let cancel_response =
                ui.add_enabled(
                    dirty,
                    egui::Button::new(
                        "Cancel"
                    ),
                );

            if cancel_response.clicked() {
                if let Some(baseline) =
                    baseline
                {
                    *configuration =
                        baseline.clone();

                    *status_message =
                        "Configuration changes discarded."
                            .to_string();
                }
            }
        },
    );


    ui.ctx()
        .data_mut(
            |data| {
                data.insert_temp(
                    selected_id,
                    selected,
                );
            }
        );
}


fn draw_nested_tab_rail(
    ui: &mut egui::Ui,
    selected: &mut ConfigurationNestedTab,
) {
    ui.vertical(
        |ui| {
            ui.set_min_width(
                200.0
            );

            for tab in
                ConfigurationNestedTab::ALL
            {
                let mut clicked =
                    false;

                ui.allocate_ui_with_layout(
                    egui::vec2(
                        194.0,
                        28.0,
                    ),
                    egui::Layout::left_to_right(
                        egui::Align::Center
                    ),
                    |ui| {
                        ui.add_space(
                            8.0
                        );

                        let response =
                            ui.selectable_label(
                                *selected == tab,
                                egui::RichText::new(
                                    tab.label()
                                )
                                .strong(),
                            );

                        if response.clicked() {
                            clicked =
                                true;
                        }
                    },
                );

                if clicked {
                    *selected =
                        tab;
                }

                ui.add_space(
                    4.0
                );
            }
        },
    );
}


fn draw_appearance(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    ui.heading("Appearance Defaults");
    ui.add_space(8.0);

    ui.checkbox(&mut configuration.show_splash, "Show splash screen");
    ui.checkbox(&mut configuration.subtitles, "Screensaver subtitles");
    ui.add_space(5.0);

    egui::Grid::new("nested_config_grid_appearance")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label("Subtitle placement:");
            egui::ComboBox::from_id_source("nested_config_subtitle_placement")
                .selected_text(configuration.subtitle_placement.as_str())
                .width(190.0)
                .show_ui(ui, |ui| {
                    for choice in [
                        "top:left", "top:center", "top:right",
                        "bottom:left", "bottom:center", "bottom:right",
                    ] {
                        ui.selectable_value(
                            &mut configuration.subtitle_placement,
                            choice.to_string(),
                            choice,
                        );
                    }
                });
            ui.end_row();
        });

    ui.add_space(5.0);
    ui.checkbox(&mut configuration.notifications, "Wallpaper Notifications");
}


fn draw_target_page(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
    policy_rows: &[PolicyDisplayRow],
    target: PolicyTarget,
    status_message: &mut String,
) {
    match target {
        PolicyTarget::Screensaver => {
            ui.heading(
                "Screensaver Settings and Defaults"
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.screensaver_enabled,
                "Enabled",
            );

            ui.add_space(
                5.0
            );

            draw_target_grid(
                ui,
                target,
                &mut configuration.screensaver_display,
                &mut configuration.screensaver_interval_seconds,
                &mut configuration.screensaver_single_policy_id,
                &mut configuration.screensaver_single_policy_name,
                policy_rows,
                Some(
                    &mut configuration.screensaver_idle_timeout_seconds
                ),
                &mut configuration.screensaver_animation_speed,
                &mut configuration.screensaver_global_texture,
                &mut configuration.screensaver_texture_primitives,
                &mut configuration.screensaver_global_palette,
                status_message,
            );
        }


        PolicyTarget::Wallpaper => {
            ui.heading(
                "Wallpaper Settings and Defaults"
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.wallpaper_enabled,
                "Enabled",
            );

            ui.add_space(
                5.0
            );

            draw_target_grid(
                ui,
                target,
                &mut configuration.wallpaper_display,
                &mut configuration.wallpaper_interval_seconds,
                &mut configuration.wallpaper_single_policy_id,
                &mut configuration.wallpaper_single_policy_name,
                policy_rows,
                None,
                &mut configuration.wallpaper_animation_speed,
                &mut configuration.wallpaper_global_texture,
                &mut configuration.wallpaper_texture_primitives,
                &mut configuration.wallpaper_global_palette,
                status_message,
            );
        }


        PolicyTarget::Unassigned => {}
    }
}


#[allow(clippy::too_many_arguments)]
fn draw_target_grid(
    ui: &mut egui::Ui,
    target: PolicyTarget,
    display_mode: &mut String,
    interval_seconds: &mut u64,
    single_policy_id: &mut Option<i64>,
    single_policy_name: &mut String,
    policy_rows: &[PolicyDisplayRow],
    idle_timeout_seconds: Option<&mut i64>,
    animation_speed: &mut f64,
    global_texture: &mut String,
    texture_primitives: &mut i64,
    global_palette: &mut String,
    status_message: &mut String,
) {
    const DEFAULT_INTERVAL_SECONDS: u64 =
        600;

    const CONTROL_WIDTH: f32 =
        190.0;

    egui::Grid::new(
        format!(
            "nested_config_grid_{:?}",
            target,
        )
    )
    .num_columns(
        2
    )
    .spacing(
        egui::vec2(
            8.0,
            6.0,
        )
    )
    .show(
        ui,
        |ui| {
            ui.label(
                "Mode:"
            );


            let previous_display_mode =
                display_mode.clone();


            egui::ComboBox::from_id_source(
                format!(
                    "nested_config_display_{:?}",
                    target,
                )
            )
            .selected_text(
                display_mode.as_str()
            )
            .width(
                CONTROL_WIDTH
            )
            .show_ui(
                ui,
                |ui| {
                    for choice in [
                        "ordered",
                        "random",
                        "single",
                    ] {
                        ui.selectable_value(
                            display_mode,
                            choice.to_string(),
                            choice,
                        );
                    }
                },
            );


            if previous_display_mode
                == "single"
                && display_mode.as_str()
                    != "single"
            {
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }


            if display_mode.as_str()
                != "single"
                && *interval_seconds == 0
            {
                *interval_seconds =
                    DEFAULT_INTERVAL_SECONDS;
            }


            ui.end_row();


            if display_mode.as_str()
                == "single"
            {
                ui.label(
                    "Policy:"
                );


                let displayed_policy =
                    if single_policy_id.is_none()
                        || single_policy_name
                            .trim()
                            .is_empty()
                    {
                        "<select policy>"
                            .to_string()
                    } else {
                        single_policy_name
                            .clone()
                    };


                ui.menu_button(
                    displayed_policy,
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(
                                320.0
                            )
                            .show(
                                ui,
                                |ui| {
                                    let mut eligible_count =
                                        0_usize;


                                    for row in policy_rows
                                        .iter()
                                        .filter(
                                            |row| {
                                                row.policy_target
                                                    == target
                                                    && row.accessible
                                            }
                                        )
                                    {
                                        eligible_count +=
                                            1;


                                        let response =
                                            ui.selectable_label(
                                                *single_policy_id
                                                    == Some(row.policy_id),
                                                row.policy_key
                                                    .as_str(),
                                            );


                                        if response.clicked() {
                                            *single_policy_id =
                                                Some(
                                                    row.policy_id
                                                );

                                            *single_policy_name =
                                                row.policy_key
                                                    .clone();

                                            *status_message =
                                                format!(
                                                    "Single {} policy selected: {}.",
                                                    target_name(
                                                        target
                                                    ),
                                                    row.policy_key,
                                                );

                                            ui.close();
                                        }
                                    }


                                    if eligible_count == 0 {
                                        ui.add_enabled(
                                            false,
                                            egui::Button::new(
                                                "No eligible policies"
                                            ),
                                        );
                                    }
                                },
                            );
                    },
                );


                ui.end_row();
            } else {
                ui.label(
                    "Interval:"
                );


                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                interval_seconds
                            )
                            .clamp_range(
                                1..=86400
                            ),
                        );

                        ui.label(
                            "seconds"
                        );
                    },
                );


                ui.end_row();
            }


            if let Some(idle_timeout_seconds) =
                idle_timeout_seconds
            {
                ui.label(
                    "Idle timeout:"
                );

                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(idle_timeout_seconds)
                            .clamp_range(1..=86400)
                    );
                    ui.label("seconds");
                });

                ui.end_row();
            }


            ui.label(
                "Animation speed:"
            );

            ui.add(
                egui::DragValue::new(animation_speed)
                    .speed(0.01)
                    .clamp_range(0.001..=100.0)
                    .suffix("x")
            );

            ui.end_row();


            ui.label(
                "Texture:"
            );

            egui::ComboBox::from_id_source(
                format!(
                    "nested_config_texture_{:?}",
                    target,
                )
            )
            .selected_text(
                global_texture.as_str()
            )
            .width(
                CONTROL_WIDTH
            )
            .show_ui(
                ui,
                |ui| {
                    ui.selectable_value(
                        global_texture,
                        "random".to_string(),
                        "random",
                    );

                    match texture_choices() {
                        Ok(choices) => {
                            for choice in choices {
                                ui.selectable_value(
                                    global_texture,
                                    choice.clone(),
                                    choice,
                                );
                            }
                        }

                        Err(error) => {
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    "Texture catalog unavailable"
                                ),
                            );

                            *status_message =
                                format!(
                                    "Unable to load texture choices: {}",
                                    error,
                                );
                        }
                    }
                },
            );

            ui.end_row();


            ui.label(
                "Palette:"
            );

            draw_curated_palette_dropdown(
                ui,
                target,
                global_palette,
                CONTROL_WIDTH,
                status_message,
            );

            ui.end_row();


            ui.label(
                "Texture primitives:"
            );

            ui.add(
                egui::DragValue::new(texture_primitives)
                    .clamp_range(1..=1024)
            );

            ui.end_row();
        },
    );


    if display_mode.as_str()
        == "single"
        && single_policy_id
            .is_none()
    {
        *status_message =
            format!(
                "Select a shader policy for Single {} display mode.",
                target_name(
                    target
                ),
            );
    }
}


fn draw_rendering_placeholders(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    const CONTROL_WIDTH: f32 = 190.0;

    ui.heading("Rendering Defaults");
    ui.add_space(8.0);

    egui::Grid::new("rendering_defaults")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label("Rendered FPS:");
            ui.add(egui::DragValue::new(&mut configuration.rendered_fps).clamp_range(16..=120));
            ui.end_row();

            ui.label("Anti-aliasing:");
            egui::ComboBox::from_id_source("rendering_default_aa")
                .selected_text(configuration.anti_aliasing.as_str())
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for choice in ["off", "fxaa"] {
                        ui.selectable_value(&mut configuration.anti_aliasing, choice.to_string(), choice);
                    }
                });
            ui.end_row();

            ui.label("Dithering:");
            egui::ComboBox::from_id_source("rendering_default_dithering")
                .selected_text(configuration.dithering.as_str())
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for choice in ["off", "subtle"] {
                        ui.selectable_value(&mut configuration.dithering, choice.to_string(), choice);
                    }
                });
            ui.end_row();

            ui.label("Color precision:");
            egui::ComboBox::from_id_source("rendering_default_precision")
                .selected_text(configuration.color_precision.as_str())
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for choice in ["auto", "standard", "high"] {
                        ui.selectable_value(&mut configuration.color_precision, choice.to_string(), choice);
                    }
                });
            ui.end_row();

            ui.label("Render scale:");
            ui.add(
                egui::DragValue::new(&mut configuration.render_scale)
                    .speed(0.05)
                    .clamp_range(0.25..=2.0)
                    .suffix("x")
            );
            ui.end_row();
        });
}


fn draw_disabled_grid(
    ui: &mut egui::Ui,
    grid_id: &str,
    rows: &[(&str, &str)],
) {
    const CONTROL_WIDTH: f32 =
        190.0;


    egui::Grid::new(
        format!(
            "nested_config_disabled_grid_{}",
            grid_id,
        )
    )
    .num_columns(
        2
    )
    .spacing(
        egui::vec2(
            8.0,
            6.0,
        )
    )
    .show(
        ui,
        |ui| {
            for (
                label,
                value,
            ) in rows
            {
                ui.label(
                    *label
                );

                ui.add_enabled(
                    false,
                    egui::Button::new(
                        *value
                    ),
                );

                ui.end_row();
            }
        },
    );
}


fn texture_choices(
) -> &'static Result<Vec<String>, String> {
    static CHOICES:
        OnceLock<
            Result<
                Vec<String>,
                String,
            >
        > =
        OnceLock::new();


    CHOICES.get_or_init(
        crate::manage_configuration::load_texture_choices
    )
}


fn curated_palette_choices(
) -> &'static Result<
    Vec<crate::manage_configuration::CuratedPaletteChoice>,
    String,
> {
    static CHOICES:
        OnceLock<
            Result<
                Vec<crate::manage_configuration::CuratedPaletteChoice>,
                String,
            >
        > =
        OnceLock::new();

    CHOICES.get_or_init(
        crate::manage_configuration::load_curated_palette_choices
    )
}


fn draw_curated_palette_dropdown(
    ui: &mut egui::Ui,
    target: PolicyTarget,
    global_palette: &mut String,
    control_width: f32,
    status_message: &mut String,
) {
    let choices = curated_palette_choices();

    let selected_text =
        if global_palette.eq_ignore_ascii_case("random") {
            "random".to_string()
        } else {
            choices
                .as_ref()
                .ok()
                .and_then(|choices| {
                    choices.iter().find(|entry| {
                        entry.color_hex.eq_ignore_ascii_case(global_palette)
                    })
                })
                .map(|entry| entry.description.clone())
                .unwrap_or_else(|| global_palette.clone())
        };

    egui::ComboBox::from_id_source(
        format!("nested_config_palette_{:?}", target)
    )
    .selected_text(selected_text)
    .width(control_width)
    .show_ui(ui, |ui| {
        let random_selected =
            global_palette.eq_ignore_ascii_case("random");

        if ui.selectable_label(random_selected, "random").clicked() {
            *global_palette = "random".to_string();
            ui.close();
        }

        match choices {
            Ok(choices) => {
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(235.0)
                    .show(ui, |ui| {
                        for entry in choices {
                            let selected =
                                entry.color_hex.eq_ignore_ascii_case(global_palette);

                            let response = curated_palette_choice_button(
                                ui,
                                entry,
                                selected,
                                control_width,
                            );

                            if response.clicked() {
                                *global_palette = entry.color_hex.clone();
                                *status_message = format!(
                                    "{} default palette selected: {}.",
                                    target_name(target),
                                    entry.description,
                                );
                                ui.close();
                            }
                        }
                    });
            }

            Err(error) => {
                ui.add_enabled(
                    false,
                    egui::Button::new("Curated palette unavailable"),
                );
                *status_message = format!(
                    "Unable to load curated palette choices: {}",
                    error,
                );
            }
        }
    });
}


fn curated_palette_choice_button(
    ui: &mut egui::Ui,
    entry: &crate::manage_configuration::CuratedPaletteChoice,
    selected: bool,
    width: f32,
) -> egui::Response {
    let row_height = ui.spacing().interact_size.y;
    let desired_size = egui::vec2(width.max(128.0), row_height);
    let (rect, response) =
        ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals =
            ui.style().interact_selectable(&response, selected);

        ui.painter().rect_filled(
            rect,
            visuals.rounding(),
            visuals.bg_fill,
        );

        ui.painter().rect_stroke(
            rect,
            visuals.rounding(),
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let swatch_size = 14.0;
        let swatch_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + 6.0,
                rect.center().y - swatch_size * 0.5,
            ),
            egui::vec2(swatch_size, swatch_size),
        );

        if let Ok(color) =
            crate::palettes::PaletteColor::parse_hex(&entry.color_hex)
        {
            ui.painter().rect_filled(
                swatch_rect,
                2.0,
                egui::Color32::from_rgb(
                    color.red(),
                    color.green(),
                    color.blue(),
                ),
            );

            ui.painter().rect_stroke(
                swatch_rect,
                2.0,
                egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.fg_stroke.color,
                ),
                egui::StrokeKind::Inside,
            );
        }

        let mut entry_font =
            egui::TextStyle::Button.resolve(ui.style());
        entry_font.size = (entry_font.size - 1.0).max(1.0);

        ui.painter().text(
            egui::pos2(
                swatch_rect.right() + 7.0,
                rect.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            &entry.description,
            entry_font,
            visuals.text_color(),
        );
    }

    response
}


fn target_name(
    target: PolicyTarget,
) -> &'static str {
    match target {
        PolicyTarget::Screensaver => "screensaver",
        PolicyTarget::Wallpaper => "wallpaper",
        PolicyTarget::Unassigned => "unassigned",
    }
}
