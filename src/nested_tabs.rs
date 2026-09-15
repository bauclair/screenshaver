// Reusable nested-tab layout helpers.
//
// This module owns reusable vertical nested-tab navigation for Control Center
// parent tabs. Configuration and Post-Processing use the same rail and page
// layout with live policy state. Persistence remains owned by editor_layout.rs /
// edit_shader.rs and the existing policy/database modules.

use std::sync::OnceLock;

use crate::editor_layout::{
    AntiAliasingSelection,
    BloomSelection,
    BulkBooleanSelection,
    ColorPrecisionSelection,
    ControlConfiguration,
    DitheringSelection,
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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PostProcessingNestedTab {
    VisualQuality,
    ImageTransforms,
    Audiovisual,
}


impl PostProcessingNestedTab {
    const ALL: [
        PostProcessingNestedTab;
        3
    ] = [
        PostProcessingNestedTab::VisualQuality,
        PostProcessingNestedTab::ImageTransforms,
        PostProcessingNestedTab::Audiovisual,
    ];


    fn label(
        self,
    ) -> &'static str {
        match self {
            PostProcessingNestedTab::VisualQuality => "Visual Quality",
            PostProcessingNestedTab::ImageTransforms => "Image Transforms",
            PostProcessingNestedTab::Audiovisual => "Audiovisual",
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
                        &ConfigurationNestedTab::ALL,
                        ConfigurationNestedTab::label,
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


fn draw_nested_tab_rail<T>(
    ui: &mut egui::Ui,
    selected: &mut T,
    tabs: &[T],
    label: impl Fn(T) -> &'static str,
)
where
    T: Copy + PartialEq,
{
    ui.vertical(
        |ui| {
            ui.set_min_width(
                200.0
            );

            for &tab in tabs {
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
                                    label(tab)
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


pub fn draw_post_processing(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    hue_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom: &mut BloomSelection,
    bloom_intensity: &mut f32,
    bloom_intensity_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_saturation: &mut f32,
    bloom_saturation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_threshold: &mut f32,
    bloom_threshold_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_rotation: &mut f32,
    bloom_frequency_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_invert: &mut bool,
    bulk_edit_mode: bool,
    bulk_invert_colors: &mut BulkBooleanSelection,
    bulk_flip_horizontal: &mut BulkBooleanSelection,
    bulk_flip_vertical: &mut BulkBooleanSelection,
    bulk_anti_aliasing_selected: &mut bool,
    bulk_dithering_selected: &mut bool,
    bulk_color_precision_selected: &mut bool,
    bulk_bloom_selected: &mut bool,
    bulk_bloom_intensity_selected: &mut bool,
    bulk_bloom_saturation_selected: &mut bool,
    bulk_bloom_threshold_selected: &mut bool,
    bulk_bloom_frequency_rotation_selected: &mut bool,
    bulk_bloom_frequency_invert: &mut BulkBooleanSelection,
    bulk_hue_rotation_selected: &mut bool,
) {
    let selected_id =
        egui::Id::new(
            "screenshaver_post_processing_nested_tab"
        );

    let mut selected =
        ui.ctx()
            .data(
                |data| {
                    data.get_temp::<PostProcessingNestedTab>(
                        selected_id
                    )
                    .unwrap_or(
                        PostProcessingNestedTab::VisualQuality
                    )
                }
            );

    let full_width =
        ui.available_width();

    let full_height =
        ui.available_height();

    ui.allocate_ui_with_layout(
        egui::vec2(
            full_width,
            full_height,
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
                        &PostProcessingNestedTab::ALL,
                        PostProcessingNestedTab::label,
                    );

                    ui.separator();
                    ui.add_space(12.0);

                    ui.vertical(
                        |ui| {
                            ui.set_width(
                                ui.available_width()
                            );

                            match selected {
                                PostProcessingNestedTab::VisualQuality => {
                                    draw_post_processing_visual_quality(
                                        ui,
                                        scale,
                                        anti_aliasing,
                                        dithering,
                                        color_precision,
                                        bulk_edit_mode,
                                        bulk_anti_aliasing_selected,
                                        bulk_dithering_selected,
                                        bulk_color_precision_selected,
                                    );
                                }

                                PostProcessingNestedTab::ImageTransforms => {
                                    draw_post_processing_image_transforms(
                                        ui,
                                        scale,
                                        shift_held,
                                        invert_colors,
                                        flip_horizontal,
                                        flip_vertical,
                                        hue_rotation,
                                        hue_rotation_drag_state,
                                        bulk_edit_mode,
                                        bulk_invert_colors,
                                        bulk_flip_horizontal,
                                        bulk_flip_vertical,
                                        bulk_hue_rotation_selected,
                                    );
                                }

                                PostProcessingNestedTab::Audiovisual => {
                                    draw_post_processing_audio(
                                        ui,
                                        scale,
                                        shift_held,
                                        bloom,
                                        bloom_intensity,
                                        bloom_intensity_drag_state,
                                        bloom_saturation,
                                        bloom_saturation_drag_state,
                                        bloom_threshold,
                                        bloom_threshold_drag_state,
                                        bloom_frequency_rotation,
                                        bloom_frequency_rotation_drag_state,
                                        bloom_frequency_invert,
                                        bulk_edit_mode,
                                        bulk_bloom_selected,
                                        bulk_bloom_intensity_selected,
                                        bulk_bloom_saturation_selected,
                                        bulk_bloom_threshold_selected,
                                        bulk_bloom_frequency_rotation_selected,
                                        bulk_bloom_frequency_invert,
                                    );
                                }
                            }
                        },
                    );
                },
            );
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


const POST_PROCESSING_CONTROL_WIDTH: f32 =
    190.0;

const POST_PROCESSING_SLIDER_WIDTH: f32 =
    240.0;


fn draw_post_processing_visual_quality(
    ui: &mut egui::Ui,
    scale: f32,
    anti_aliasing: &mut AntiAliasingSelection,
    dithering: &mut DitheringSelection,
    color_precision: &mut ColorPrecisionSelection,
    bulk_edit_mode: bool,
    bulk_anti_aliasing_selected: &mut bool,
    bulk_dithering_selected: &mut bool,
    bulk_color_precision_selected: &mut bool,
) {
    ui.heading("Visual Quality");
    ui.add_space(8.0);

    egui::Grid::new(
        "post_processing_visual_quality_grid"
    )
    .num_columns(2)
    .spacing(egui::vec2(8.0, 8.0))
    .show(
        ui,
        |ui| {
            ui.label("Anti-Aliasing:")
                .on_hover_text(
                    "Controls edge smoothing for the rendered shader."
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_anti_aliasing_selected
                {
                    "Unchanged"
                } else {
                    match *anti_aliasing {
                        AntiAliasingSelection::Off => "Off",
                        AntiAliasingSelection::Fxaa => "FXAA",
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_anti_aliasing"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_anti_aliasing_selected,
                            "Unchanged",
                        ).clicked() {
                            *bulk_anti_aliasing_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        anti_aliasing,
                        AntiAliasingSelection::Off,
                        "Off",
                    ).clicked() && bulk_edit_mode {
                        *bulk_anti_aliasing_selected = true;
                    }

                    if ui.selectable_value(
                        anti_aliasing,
                        AntiAliasingSelection::Fxaa,
                        "FXAA",
                    ).clicked() && bulk_edit_mode {
                        *bulk_anti_aliasing_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_anti_aliasing_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();

            ui.label("Dithering:")
                .on_hover_text(
                    "Controls subtle dithering used to reduce visible color banding."
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_dithering_selected
                {
                    "Unchanged"
                } else {
                    match *dithering {
                        DitheringSelection::Off => "Off",
                        DitheringSelection::Subtle => "Subtle",
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_dithering"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_dithering_selected,
                            "Unchanged",
                        ).clicked() {
                            *bulk_dithering_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Off,
                        "Off",
                    ).clicked() && bulk_edit_mode {
                        *bulk_dithering_selected = true;
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Subtle,
                        "Subtle",
                    ).clicked() && bulk_edit_mode {
                        *bulk_dithering_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_dithering_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();

            ui.label("Color Precision:")
                .on_hover_text(
                    "Selects the color precision used by post-processing."
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_color_precision_selected
                {
                    "Unchanged"
                } else {
                    match *color_precision {
                        ColorPrecisionSelection::Automatic => "Automatic",
                        ColorPrecisionSelection::Standard => "Standard Precision",
                        ColorPrecisionSelection::High => "High Precision",
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_color_precision"
            )
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_color_precision_selected,
                            "Unchanged",
                        ).clicked() {
                            *bulk_color_precision_selected = false;
                        }
                        ui.separator();
                    }

                    for (choice, label) in [
                        (ColorPrecisionSelection::Automatic, "Automatic"),
                        (ColorPrecisionSelection::Standard, "Standard Precision"),
                        (ColorPrecisionSelection::High, "High Precision"),
                    ] {
                        if ui.selectable_value(
                            color_precision,
                            choice,
                            label,
                        ).clicked() && bulk_edit_mode {
                            *bulk_color_precision_selected = true;
                        }
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_color_precision_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }

            ui.end_row();
        },
    );
}


fn draw_post_processing_image_transforms(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    invert_colors: &mut bool,
    flip_horizontal: &mut bool,
    flip_vertical: &mut bool,
    hue_rotation: &mut f32,
    hue_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_invert_colors: &mut BulkBooleanSelection,
    bulk_flip_horizontal: &mut BulkBooleanSelection,
    bulk_flip_vertical: &mut BulkBooleanSelection,
    bulk_hue_rotation_selected: &mut bool,
) {
    ui.heading("Image Transforms");
    ui.add_space(8.0);

    if bulk_edit_mode {
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_invert_colors",
            scale,
            "Invert Colors",
            bulk_invert_colors,
            "Inverts the final rendered colors.",
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_horizontal",
            scale,
            "Flip Horizontal",
            bulk_flip_horizontal,
            "Mirrors the final image horizontally.",
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_vertical",
            scale,
            "Flip Vertical",
            bulk_flip_vertical,
            "Mirrors the final image vertically.",
        );
    } else {
        ui.checkbox(
            invert_colors,
            "Invert Colors",
        )
        .on_hover_text(
            "Inverts the final rendered colors."
        );

        ui.checkbox(
            flip_horizontal,
            "Flip Horizontal",
        )
        .on_hover_text(
            "Mirrors the final image horizontally."
        );

        ui.checkbox(
            flip_vertical,
            "Flip Vertical",
        )
        .on_hover_text(
            "Mirrors the final image vertically."
        );
    }

    ui.add_space(10.0);

    draw_numeric_slider_row(
        ui,
        "Hue Rotation:",
        hue_rotation,
        crate::postprocess_shader::HUE_ROTATION_MIN,
        crate::postprocess_shader::HUE_ROTATION_MAX,
        "°",
        shift_held,
        hue_rotation_drag_state,
        bulk_edit_mode,
        bulk_hue_rotation_selected,
        true,
        scale,
        "Rotates the displayed shader colors around the hue wheel.",
    );
}


fn draw_post_processing_audio(
    ui: &mut egui::Ui,
    scale: f32,
    shift_held: bool,
    bloom: &mut BloomSelection,
    bloom_intensity: &mut f32,
    bloom_intensity_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_saturation: &mut f32,
    bloom_saturation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_threshold: &mut f32,
    bloom_threshold_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_rotation: &mut f32,
    bloom_frequency_rotation_drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bloom_frequency_invert: &mut bool,
    bulk_edit_mode: bool,
    bulk_bloom_selected: &mut bool,
    bulk_bloom_intensity_selected: &mut bool,
    bulk_bloom_saturation_selected: &mut bool,
    bulk_bloom_threshold_selected: &mut bool,
    bulk_bloom_frequency_rotation_selected: &mut bool,
    bulk_bloom_frequency_invert: &mut BulkBooleanSelection,
) {
    ui.heading("Audiovisual");
    ui.add_space(8.0);

    ui.horizontal(
        |ui| {
            ui.label("Audiovisual Effect:")
                .on_hover_text(
                    "Selects the audio-driven post-processing effect: Off, Audio Bloom, Spectral Bloom, or experimental Loudness Bloom."
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_bloom_selected
                {
                    "Unchanged"
                } else {
                    match *bloom {
                        BloomSelection::Off => "Off",
                        BloomSelection::Audio => "Audio Bloom",
                        BloomSelection::Spectral => "Spectral Bloom",
                        BloomSelection::Loudness => "Loudness Bloom",
                    }
                };

            let response =
                egui::ComboBox::from_id_source(
                "post_processing_bloom_mode"
            )
            .selected_text(selected_text)
            .width(150.0)
            .show_ui(
                ui,
                |ui| {
                    if bulk_edit_mode {
                        if ui.selectable_label(
                            !*bulk_bloom_selected,
                            "Unchanged",
                        ).clicked() {
                            *bulk_bloom_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Off,
                        "Off",
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Audio,
                        "Audio Bloom",
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Spectral,
                        "Spectral Bloom",
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Loudness,
                        "Loudness Bloom",
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }
                },
            )
                .response;

            if bulk_edit_mode
                && *bulk_bloom_selected
            {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }
        },
    );

    ui.add_space(8.0);

    // In ordinary editing, Off disables dependent Bloom parameters.
    // In Bulk Edit, an Unchanged Bloom Mode still permits independently
    // applying numeric Bloom values across the selected policies.
    let bloom_controls_available =
        if bulk_edit_mode
            && !*bulk_bloom_selected
        {
            true
        } else {
            *bloom != BloomSelection::Off
        };

    let frequency_controls_available =
        if bulk_edit_mode
            && !*bulk_bloom_selected
        {
            true
        } else {
            bloom_controls_available
                && *bloom != BloomSelection::Loudness
        };

    egui::Grid::new(
        "post_processing_audio_grid"
    )
    .num_columns(2)
    .spacing(egui::vec2(8.0, 8.0))
    .show(
        ui,
        |ui| {
            draw_numeric_slider_grid_row(
                ui,
                "Bloom Intensity:",
                bloom_intensity,
                crate::render_bloom::BLOOM_INTENSITY_MIN,
                crate::render_bloom::BLOOM_INTENSITY_MAX,
                "",
                shift_held,
                bloom_intensity_drag_state,
                bulk_edit_mode,
                bulk_bloom_intensity_selected,
                bloom_controls_available,
                scale,
                "Controls the strength of the selected Bloom mode.",
            );

            draw_numeric_slider_grid_row(
                ui,
                "Bloom Saturation:",
                bloom_saturation,
                crate::render_bloom::BLOOM_SATURATION_MIN,
                crate::render_bloom::BLOOM_SATURATION_MAX,
                "",
                shift_held,
                bloom_saturation_drag_state,
                bulk_edit_mode,
                bulk_bloom_saturation_selected,
                bloom_controls_available,
                scale,
                "Boosts bloom color saturation from the neutral 1.0 level up to 2.0 without changing the displayed shader colors.",
            );

            draw_numeric_slider_grid_row(
                ui,
                "Bloom Threshold:",
                bloom_threshold,
                crate::render_bloom::BLOOM_THRESHOLD_MIN,
                crate::render_bloom::BLOOM_THRESHOLD_MAX,
                "",
                shift_held,
                bloom_threshold_drag_state,
                bulk_edit_mode,
                bulk_bloom_threshold_selected,
                bloom_controls_available,
                scale,
                "Controls the brightness threshold used to extract bloom.",
            );

            draw_numeric_slider_grid_row(
                ui,
                "Frequency Rotation:",
                bloom_frequency_rotation,
                crate::render_bloom::BLOOM_FREQUENCY_ROTATION_MIN,
                crate::render_bloom::BLOOM_FREQUENCY_ROTATION_MAX,
                "°",
                shift_held,
                bloom_frequency_rotation_drag_state,
                bulk_edit_mode,
                bulk_bloom_frequency_rotation_selected,
                frequency_controls_available,
                scale,
                "Rotates Audio/Spectral frequency-to-color mapping. Disabled for Loudness Bloom.",
            );
        },
    );

    ui.add_space(8.0);

    ui.add_enabled_ui(
        frequency_controls_available,
        |ui| {
            if bulk_edit_mode {
                draw_bulk_boolean_row(
                    ui,
                    "post_processing_bulk_bloom_frequency_invert",
                    scale,
                    "Invert Frequency Mapping",
                    bulk_bloom_frequency_invert,
                    "Reverses Audio/Spectral low-to-high color-frequency mapping. Disabled for Loudness Bloom.",
                );
            } else {
                ui.checkbox(
                    bloom_frequency_invert,
                    "Invert Frequency Mapping",
                )
                .on_hover_text(
                    "Reverses Audio/Spectral low-to-high color-frequency mapping. Disabled for Loudness Bloom."
                );
            }
        },
    );
}


fn draw_bulk_boolean_row(
    ui: &mut egui::Ui,
    id: &'static str,
    scale: f32,
    label: &'static str,
    selection: &mut BulkBooleanSelection,
    help: &'static str,
) {
    ui.horizontal(
        |ui| {
            ui.label(label)
                .on_hover_text(help);

            let selected_text =
                match *selection {
                    BulkBooleanSelection::Unchanged => "Unchanged",
                    BulkBooleanSelection::True => "Enabled",
                    BulkBooleanSelection::False => "Disabled",
                };

            let response =
                egui::ComboBox::from_id_source(id)
                .selected_text(selected_text)
                .width(POST_PROCESSING_CONTROL_WIDTH)
                .show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::Unchanged,
                            "Unchanged",
                        );
                        ui.separator();
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::True,
                            "Enabled",
                        );
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::False,
                            "Disabled",
                        );
                    },
                )
                .response;


            if !matches!(
                *selection,
                BulkBooleanSelection::Unchanged
            ) {
                crate::editor_theme::paint_bulk_edit_border(
                    ui,
                    response.rect,
                    scale,
                );
            }
        },
    );
}


fn draw_numeric_slider_row(
    ui: &mut egui::Ui,
    label: &'static str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
    help: &'static str,
) {
    ui.horizontal(
        |ui| {
            ui.label(label)
                .on_hover_text(help);
            draw_numeric_slider_control(
                ui,
                value,
                minimum,
                maximum,
                suffix,
                shift_held,
                drag_state,
                bulk_edit_mode,
                bulk_selected,
                control_available,
                scale,
            );
        },
    );
}


fn draw_numeric_slider_grid_row(
    ui: &mut egui::Ui,
    label: &'static str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
    help: &'static str,
) {
    ui.label(label)
        .on_hover_text(help);
    draw_numeric_slider_control(
        ui,
        value,
        minimum,
        maximum,
        suffix,
        shift_held,
        drag_state,
        bulk_edit_mode,
        bulk_selected,
        control_available,
        scale,
    );
    ui.end_row();
}


fn draw_numeric_slider_control(
    ui: &mut egui::Ui,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    suffix: &'static str,
    shift_held: bool,
    drag_state: &mut Option<crate::editor_layout::SliderDragState>,
    bulk_edit_mode: bool,
    bulk_selected: &mut bool,
    control_available: bool,
    scale: f32,
) {
    ui.horizontal(
        |ui| {
            ui.spacing_mut().slider_width =
                POST_PROCESSING_SLIDER_WIDTH;

            let slider_enabled =
                control_available
                    && (
                        !bulk_edit_mode
                            || *bulk_selected
                    );

            ui.add_enabled_ui(
                slider_enabled,
                |ui| {
                    // Reuse the Control Center's established fine-drag slider
                    // behavior. Holding Shift reduces drag sensitivity to 0.1x
                    // (10x finer), and the helper re-anchors if Shift changes
                    // while the pointer is already dragging.
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            POST_PROCESSING_SLIDER_WIDTH,
                            ui.spacing().interact_size.y,
                        ),
                        egui::Layout::left_to_right(
                            egui::Align::Center
                        ),
                        |ui| {
                            ui.set_width(
                                POST_PROCESSING_SLIDER_WIDTH
                            );

                            crate::editor_layout::draw_fine_slider(
                                ui,
                                value,
                                minimum,
                                maximum,
                                shift_held,
                                scale,
                                drag_state,
                            );
                        },
                    );
                },
            );

            let displayed_value =
                if suffix == "°" {
                    format!("{:.1}°", *value)
                } else {
                    format!("{:.2}", *value)
                };

            if bulk_edit_mode {
                let response =
                    ui.add_enabled(
                        control_available,
                        egui::Button::new(displayed_value),
                    )
                    .on_hover_cursor(
                        egui::CursorIcon::PointingHand
                    )
                    .on_hover_text(
                        if *bulk_selected {
                            "Click to exclude this value from Bulk Edit"
                        } else {
                            "Click to include this value in Bulk Edit"
                        }
                    );

                if response.clicked() {
                    *bulk_selected =
                        !*bulk_selected;
                }

                if *bulk_selected {
                    crate::editor_theme::paint_bulk_edit_border(
                        ui,
                        response.rect,
                        scale,
                    );
                }
            } else {
                ui.label(displayed_value);
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
                None,
                &mut configuration.screensaver_display,
                &mut configuration.screensaver_interval_seconds,
                &mut configuration.screensaver_single_policy_id,
                &mut configuration.screensaver_single_policy_name,
                policy_rows,
                Some(
                    &mut configuration.screensaver_idle_timeout_value
                ),
                Some(
                    &mut configuration.screensaver_idle_timeout_unit
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
                Some(
                    &mut configuration.wallpaper_display_format
                ),
                &mut configuration.wallpaper_display,
                &mut configuration.wallpaper_interval_seconds,
                &mut configuration.wallpaper_single_policy_id,
                &mut configuration.wallpaper_single_policy_name,
                policy_rows,
                None,
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
    wallpaper_display_format: Option<
        &mut crate::manage_configuration::WallpaperDisplayFormat
    >,
    display_mode: &mut String,
    interval_seconds: &mut u64,
    single_policy_id: &mut Option<i64>,
    single_policy_name: &mut String,
    policy_rows: &[PolicyDisplayRow],
    idle_timeout_value: Option<&mut i64>,
    idle_timeout_unit: Option<&mut String>,
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
            if let Some(
                wallpaper_display_format
            ) = wallpaper_display_format
            {
                ui.label(
                    "Display Format:"
                )
                .on_hover_text(
                    "Selects whether wallpaper is presented full-screen or in a normal desktop-managed window."
                );

                egui::ComboBox::from_id_source(
                    "nested_config_wallpaper_display_format_combo"
                )
                .selected_text(
                    match *wallpaper_display_format {
                        crate::manage_configuration::WallpaperDisplayFormat::FullScreen => {
                            "Full-screen"
                        }

                        crate::manage_configuration::WallpaperDisplayFormat::Windowed => {
                            "Windowed"
                        }
                    }
                )
                .width(
                    CONTROL_WIDTH
                )
                .show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(
                            wallpaper_display_format,
                            crate::manage_configuration::WallpaperDisplayFormat::FullScreen,
                            "Full-screen",
                        );

                        ui.selectable_value(
                            wallpaper_display_format,
                            crate::manage_configuration::WallpaperDisplayFormat::Windowed,
                            "Windowed",
                        );
                    },
                );

                ui.end_row();
            }


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


            if let (
                Some(idle_timeout_value),
                Some(idle_timeout_unit),
            ) = (
                idle_timeout_value,
                idle_timeout_unit,
            ) {
                ui.label(
                    "Idle timeout:"
                );

                ui.horizontal(
                    |ui| {
                        ui.add(
                            egui::DragValue::new(
                                idle_timeout_value
                            )
                            .clamp_range(
                                1..=86400
                            )
                        );

                        egui::ComboBox::from_id_source(
                            "nested_config_idle_timeout_unit"
                        )
                        .selected_text(
                            match idle_timeout_unit.as_str() {
                                "seconds" => "Seconds",
                                "minutes" => "Minutes",
                                "hours" => "Hours",
                                _ => "Seconds",
                            }
                        )
                        .width(
                            90.0
                        )
                        .show_ui(
                            ui,
                            |ui| {
                                for (
                                    value,
                                    label,
                                ) in [
                                    (
                                        "seconds",
                                        "Seconds",
                                    ),
                                    (
                                        "minutes",
                                        "Minutes",
                                    ),
                                    (
                                        "hours",
                                        "Hours",
                                    ),
                                ] {
                                    ui.selectable_value(
                                        idle_timeout_unit,
                                        value.to_string(),
                                        label,
                                    );
                                }
                            },
                        );
                    },
                );

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
