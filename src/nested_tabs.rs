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
    Lyrics,
    DataIo,
}


impl ConfigurationNestedTab {
    const ALL: [
        ConfigurationNestedTab;
        6
    ] = [
        ConfigurationNestedTab::Appearance,
        ConfigurationNestedTab::Screensaver,
        ConfigurationNestedTab::Wallpaper,
        ConfigurationNestedTab::Rendering,
        ConfigurationNestedTab::Lyrics,
        ConfigurationNestedTab::DataIo,
    ];


    fn label(
        self,
    ) -> String {
        match self {
            ConfigurationNestedTab::Appearance => {
                crate::manage_localization::runtime_text("tab.appearance")
            }

            ConfigurationNestedTab::Screensaver => {
                crate::manage_localization::runtime_text(
                    "target.screensaver"
                )
            }

            ConfigurationNestedTab::Wallpaper => {
                crate::manage_localization::runtime_text(
                    "target.wallpaper"
                )
            }

            ConfigurationNestedTab::Rendering => {
                crate::manage_localization::runtime_text("tab.rendering")
            }

            ConfigurationNestedTab::Lyrics => {
                crate::manage_localization::runtime_text("tab.lyrics")
            }

            ConfigurationNestedTab::DataIo => {
                crate::manage_localization::runtime_text("tab.data_io")
            }
        }
    }
}



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PostProcessingNestedTab {
    VisualQuality,
    ImageTransforms,
    Audiovisual,
    AudioMotion,
}


impl PostProcessingNestedTab {
    const ALL: [
        PostProcessingNestedTab;
        4
    ] = [
        PostProcessingNestedTab::VisualQuality,
        PostProcessingNestedTab::ImageTransforms,
        PostProcessingNestedTab::Audiovisual,
        PostProcessingNestedTab::AudioMotion,
    ];


    fn label(
        self,
    ) -> String {
        match self {
            PostProcessingNestedTab::VisualQuality => crate::manage_localization::runtime_text("post.tab.visual_quality"),
            PostProcessingNestedTab::ImageTransforms => crate::manage_localization::runtime_text("post.tab.image_transforms"),
            PostProcessingNestedTab::Audiovisual => crate::manage_localization::runtime_text("post.tab.audiovisual"),
            PostProcessingNestedTab::AudioMotion => crate::manage_localization::runtime_text("post.tab.audio_motion"),
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
    export_destination_browse_requested: &mut Option<std::path::PathBuf>,
    import_archive_browse_requested: &mut Option<std::path::PathBuf>,
) {
    let Some(configuration) =
        configuration.as_mut()
    else {
        ui.label(
            egui::RichText::new(
                crate::manage_localization::runtime_text("config.unavailable")
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

                                ConfigurationNestedTab::Lyrics => {
                                    draw_lyrics(
                                        ui,
                                        configuration,
                                    );
                                }

                                ConfigurationNestedTab::DataIo => {
                                    draw_data_io_shell(
                                        ui,
                                        status_message,
                                        export_destination_browse_requested,
                                        import_archive_browse_requested,
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
                        crate::manage_localization::runtime_text("config.save")
                    ),
                );

            if save_response.clicked() {
                *save_requested =
                    true;

                *status_message =
                    crate::manage_localization::runtime_text("config.saving");
            }


            ui.add_space(
                8.0
            );


            let cancel_response =
                ui.add_enabled(
                    dirty,
                    egui::Button::new(
                        crate::manage_localization::runtime_text("common.cancel")
                    ),
                );

            if cancel_response.clicked() {
                if let Some(baseline) =
                    baseline
                {
                    *configuration =
                        baseline.clone();

                    *status_message =
                        crate::manage_localization::runtime_text("config.discarded");
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


fn draw_data_io_shell(
    ui: &mut egui::Ui,
    status_message: &mut String,
    export_destination_browse_requested: &mut Option<std::path::PathBuf>,
    import_archive_browse_requested: &mut Option<std::path::PathBuf>,
) {
    ui.heading(
        crate::manage_localization::runtime_text("data_io.heading")
    );

    ui.add_space(
        8.0
    );

    ui.label(
        crate::manage_localization::runtime_text("data_io.description")
    );

    ui.add_space(
        12.0
    );

    crate::manage_backup::draw_controls(
        ui,
        status_message,
    );

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);

    ui.label(egui::RichText::new(crate::manage_localization::runtime_text("data_io.portable_data")).strong());
    ui.add_space(6.0);

    ui.horizontal(
        |ui| {
            if ui.button(
                crate::manage_localization::runtime_text("data_io.import")
            )
            .clicked()
            {
                crate::import_data::open(
                    ui.ctx()
                );
            }

            ui.add_space(
                8.0
            );

            if ui.button(
                crate::manage_localization::runtime_text("data_io.export")
            )
            .clicked()
            {
                crate::export_data::open(
                    ui.ctx()
                );
            }
        },
    );


    crate::export_data::draw(
        ui.ctx(),
        export_destination_browse_requested,
    );

    crate::import_data::draw(
        ui.ctx(),
        import_archive_browse_requested,
    );
}


fn draw_nested_tab_rail<T>(
    ui: &mut egui::Ui,
    selected: &mut T,
    tabs: &[T],
    label: impl Fn(T) -> String,
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
    audio_motion: &mut crate::render_audio_motion::AudioMotionEffect,
    bulk_audio_motion_selected: &mut bool,
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

                                PostProcessingNestedTab::AudioMotion => {
                                    draw_post_processing_audio_motion(
                                        ui,
                                        scale,
                                        audio_motion,
                                        bulk_edit_mode,
                                        bulk_audio_motion_selected,
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


fn draw_post_processing_audio_motion(
    ui: &mut egui::Ui,
    scale: f32,
    audio_motion: &mut crate::render_audio_motion::AudioMotionEffect,
    bulk_edit_mode: bool,
    bulk_audio_motion_selected: &mut bool,
) {
    ui.heading(crate::manage_localization::runtime_text("post.tab.audio_motion"));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label(crate::manage_localization::runtime_text("post.motion.effect"))
            .on_hover_text(crate::manage_localization::runtime_text("post.motion.effect_help"));

        let selected_text = if bulk_edit_mode && !*bulk_audio_motion_selected {
            crate::manage_localization::runtime_text("post.common.unchanged")
        } else {
            audio_motion.display_name().to_string()
        };

        let response = egui::ComboBox::from_id_source("post_processing_audio_motion_effect")
            .selected_text(selected_text)
            .width(POST_PROCESSING_CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                if bulk_edit_mode {
                    if ui.selectable_label(!*bulk_audio_motion_selected, crate::manage_localization::runtime_text("post.common.unchanged")).clicked() {
                        *bulk_audio_motion_selected = false;
                    }
                    ui.separator();
                }

                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::Off, crate::manage_localization::runtime_text("post.common.off")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::WooferFromHell, crate::manage_localization::runtime_text("post.motion.woofer")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::FftMirrorWarp, crate::manage_localization::runtime_text("post.motion.fft_mirror")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
                if ui.selectable_value(audio_motion, crate::render_audio_motion::AudioMotionEffect::PolarPropeller, crate::manage_localization::runtime_text("post.motion.polar_propeller")).clicked() && bulk_edit_mode {
                    *bulk_audio_motion_selected = true;
                }
            })
            .response;

        if bulk_edit_mode && *bulk_audio_motion_selected {
            crate::editor_theme::paint_bulk_edit_border(ui, response.rect, scale);
        }
    });
}


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
    ui.heading(crate::manage_localization::runtime_text("post.tab.visual_quality"));
    ui.add_space(8.0);

    egui::Grid::new(
        "post_processing_visual_quality_grid"
    )
    .num_columns(2)
    .spacing(egui::vec2(8.0, 8.0))
    .show(
        ui,
        |ui| {
            ui.label(crate::manage_localization::runtime_text("post.visual.anti_aliasing"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.anti_aliasing_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_anti_aliasing_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *anti_aliasing {
                        AntiAliasingSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        AntiAliasingSelection::Fxaa => "FXAA".to_string(),
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
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_anti_aliasing_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        anti_aliasing,
                        AntiAliasingSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
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

            ui.label(crate::manage_localization::runtime_text("rendering.dithering"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.dithering_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_dithering_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *dithering {
                        DitheringSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        DitheringSelection::Subtle => crate::manage_localization::runtime_text("post.visual.subtle"),
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
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_dithering_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_dithering_selected = true;
                    }

                    if ui.selectable_value(
                        dithering,
                        DitheringSelection::Subtle,
                        crate::manage_localization::runtime_text("post.visual.subtle"),
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

            ui.label(crate::manage_localization::runtime_text("post.visual.color_precision"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.visual.color_precision_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_color_precision_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *color_precision {
                        ColorPrecisionSelection::Automatic => crate::manage_localization::runtime_text("post.visual.automatic"),
                        ColorPrecisionSelection::Standard => crate::manage_localization::runtime_text("post.visual.standard_precision"),
                        ColorPrecisionSelection::High => crate::manage_localization::runtime_text("post.visual.high_precision"),
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
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_color_precision_selected = false;
                        }
                        ui.separator();
                    }

                    for (choice, label) in [
                        (ColorPrecisionSelection::Automatic, crate::manage_localization::runtime_text("post.visual.automatic")),
                        (ColorPrecisionSelection::Standard, crate::manage_localization::runtime_text("post.visual.standard_precision")),
                        (ColorPrecisionSelection::High, crate::manage_localization::runtime_text("post.visual.high_precision")),
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
    ui.heading(crate::manage_localization::runtime_text("post.tab.image_transforms"));
    ui.add_space(8.0);

    if bulk_edit_mode {
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_invert_colors",
            scale,
            crate::manage_localization::runtime_text("post.transform.invert_colors"),
            bulk_invert_colors,
            crate::manage_localization::runtime_text("post.transform.invert_colors_help"),
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_horizontal",
            scale,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal"),
            bulk_flip_horizontal,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal_help"),
        );
        draw_bulk_boolean_row(
            ui,
            "post_processing_bulk_flip_vertical",
            scale,
            crate::manage_localization::runtime_text("post.transform.flip_vertical"),
            bulk_flip_vertical,
            crate::manage_localization::runtime_text("post.transform.flip_vertical_help"),
        );
    } else {
        ui.checkbox(
            invert_colors,
            crate::manage_localization::runtime_text("post.transform.invert_colors"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.invert_colors_help")
        );

        ui.checkbox(
            flip_horizontal,
            crate::manage_localization::runtime_text("post.transform.flip_horizontal"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.flip_horizontal_help")
        );

        ui.checkbox(
            flip_vertical,
            crate::manage_localization::runtime_text("post.transform.flip_vertical"),
        )
        .on_hover_text(
            crate::manage_localization::runtime_text("post.transform.flip_vertical_help")
        );
    }

    ui.add_space(10.0);

    draw_numeric_slider_row(
        ui,
        crate::manage_localization::runtime_text("post.transform.hue_rotation"),
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
        crate::manage_localization::runtime_text("post.transform.hue_rotation_help"),
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
    ui.heading(crate::manage_localization::runtime_text("post.tab.audiovisual"));
    ui.add_space(8.0);

    ui.horizontal(
        |ui| {
            ui.label(crate::manage_localization::runtime_text("post.audio.effect"))
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.audio.effect_help")
                );

            let selected_text =
                if bulk_edit_mode
                    && !*bulk_bloom_selected
                {
                    crate::manage_localization::runtime_text("post.common.unchanged")
                } else {
                    match *bloom {
                        BloomSelection::Off => crate::manage_localization::runtime_text("post.common.off"),
                        BloomSelection::Audio => crate::manage_localization::runtime_text("post.audio.audio_bloom"),
                        BloomSelection::Spectral => crate::manage_localization::runtime_text("post.audio.spectral_bloom"),
                        BloomSelection::Loudness => crate::manage_localization::runtime_text("post.audio.loudness_bloom"),
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
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        ).clicked() {
                            *bulk_bloom_selected = false;
                        }
                        ui.separator();
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Off,
                        crate::manage_localization::runtime_text("post.common.off"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }

                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Audio,
                        crate::manage_localization::runtime_text("post.audio.audio_bloom"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Spectral,
                        crate::manage_localization::runtime_text("post.audio.spectral_bloom"),
                    ).clicked() && bulk_edit_mode {
                        *bulk_bloom_selected = true;
                    }


                    if ui.selectable_value(
                        bloom,
                        BloomSelection::Loudness,
                        crate::manage_localization::runtime_text("post.audio.loudness_bloom"),
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
                crate::manage_localization::runtime_text("post.audio.bloom_intensity"),
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
                crate::manage_localization::runtime_text("post.audio.bloom_intensity_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.bloom_saturation"),
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
                crate::manage_localization::runtime_text("post.audio.bloom_saturation_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.bloom_threshold"),
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
                crate::manage_localization::runtime_text("post.audio.bloom_threshold_help"),
            );

            draw_numeric_slider_grid_row(
                ui,
                crate::manage_localization::runtime_text("post.audio.frequency_rotation"),
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
                crate::manage_localization::runtime_text("post.audio.frequency_rotation_help"),
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
                    crate::manage_localization::runtime_text("post.audio.invert_frequency"),
                    bulk_bloom_frequency_invert,
                    crate::manage_localization::runtime_text("post.audio.invert_frequency_help"),
                );
            } else {
                ui.checkbox(
                    bloom_frequency_invert,
                    crate::manage_localization::runtime_text("post.audio.invert_frequency"),
                )
                .on_hover_text(
                    crate::manage_localization::runtime_text("post.audio.invert_frequency_help")
                );
            }
        },
    );
}


fn draw_bulk_boolean_row(
    ui: &mut egui::Ui,
    id: &'static str,
    scale: f32,
    label: String,
    selection: &mut BulkBooleanSelection,
    help: String,
) {
    ui.horizontal(
        |ui| {
            ui.label(label)
                .on_hover_text(help);

            let selected_text =
                match *selection {
                    BulkBooleanSelection::Unchanged => crate::manage_localization::runtime_text("post.common.unchanged"),
                    BulkBooleanSelection::True => crate::manage_localization::runtime_text("post.common.enabled"),
                    BulkBooleanSelection::False => crate::manage_localization::runtime_text("post.common.disabled"),
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
                            crate::manage_localization::runtime_text("post.common.unchanged"),
                        );
                        ui.separator();
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::True,
                            crate::manage_localization::runtime_text("common.enabled"),
                        );
                        ui.selectable_value(
                            selection,
                            BulkBooleanSelection::False,
                            crate::manage_localization::runtime_text("post.common.disabled"),
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
    label: String,
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
    help: String,
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
    label: String,
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
    help: String,
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
                            crate::manage_localization::runtime_text("post.bulk.exclude")
                        } else {
                            crate::manage_localization::runtime_text("post.bulk.include")
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
    ui.heading(crate::manage_localization::runtime_text("appearance.heading"));
    ui.add_space(8.0);

    ui.checkbox(&mut configuration.show_splash, crate::manage_localization::runtime_text("appearance.show_splash"));
    ui.checkbox(&mut configuration.subtitles, crate::manage_localization::runtime_text("appearance.screensaver_subtitles"));
    ui.add_space(5.0);

    egui::Grid::new("nested_config_grid_appearance")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label(crate::manage_localization::runtime_text("appearance.subtitle_placement"));
            egui::ComboBox::from_id_source("nested_config_subtitle_placement")
                .selected_text(
                    match configuration.subtitle_placement.as_str() {
                        "top:left" => crate::manage_localization::runtime_text("placement.top_left"),
                        "top:center" => crate::manage_localization::runtime_text("placement.top_center"),
                        "top:right" => crate::manage_localization::runtime_text("placement.top_right"),
                        "bottom:left" => crate::manage_localization::runtime_text("placement.bottom_left"),
                        "bottom:center" => crate::manage_localization::runtime_text("placement.bottom_center"),
                        "bottom:right" => crate::manage_localization::runtime_text("placement.bottom_right"),
                        _ => configuration.subtitle_placement.clone(),
                    }
                )
                .width(190.0)
                .show_ui(ui, |ui| {
                    for (choice, label_key) in [
                        ("top:left", "placement.top_left"),
                        ("top:center", "placement.top_center"),
                        ("top:right", "placement.top_right"),
                        ("bottom:left", "placement.bottom_left"),
                        ("bottom:center", "placement.bottom_center"),
                        ("bottom:right", "placement.bottom_right"),
                    ] {
                        ui.selectable_value(
                            &mut configuration.subtitle_placement,
                            choice.to_string(),
                            crate::manage_localization::runtime_text(label_key),
                        );
                    }
                });
            ui.end_row();
        });

    ui.add_space(5.0);
    ui.checkbox(&mut configuration.notifications, crate::manage_localization::runtime_text("appearance.wallpaper_notifications"));
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
                crate::manage_localization::runtime_text("target.screensaver_settings")
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.screensaver_enabled,
                crate::manage_localization::runtime_text("common.enabled"),
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
                &mut configuration.screensaver_playlist_id,
                &mut configuration.screensaver_playlist_name,
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
                crate::manage_localization::runtime_text("target.wallpaper_settings")
            );

            ui.add_space(
                8.0
            );

            ui.checkbox(
                &mut configuration.wallpaper_enabled,
                crate::manage_localization::runtime_text("common.enabled"),
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
                &mut configuration.wallpaper_playlist_id,
                &mut configuration.wallpaper_playlist_name,
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
    playlist_id: &mut Option<i64>,
    playlist_name: &mut String,
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
                    crate::manage_localization::runtime_text("target.display_format")
                )
                .on_hover_text(
                    crate::manage_localization::runtime_text("target.display_format_help")
                );

                egui::ComboBox::from_id_source(
                    "nested_config_wallpaper_display_format_combo"
                )
                .selected_text(
                    match *wallpaper_display_format {
                        crate::manage_configuration::WallpaperDisplayFormat::FullScreen => { crate::manage_localization::runtime_text("target.full_screen") }

                        crate::manage_configuration::WallpaperDisplayFormat::Windowed => { crate::manage_localization::runtime_text("target.windowshader") }
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
                            crate::manage_localization::runtime_text("target.full_screen"),
                        );

                        ui.selectable_value(
                            wallpaper_display_format,
                            crate::manage_configuration::WallpaperDisplayFormat::Windowed,
                            crate::manage_localization::runtime_text("target.windowshader"),
                        );
                    },
                );

                ui.end_row();
            }


            ui.label(
                crate::manage_localization::runtime_text("target.mode")
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
                match display_mode.as_str() {
                    "ordered" => crate::manage_localization::runtime_text("mode.ordered"),
                    "random" => crate::manage_localization::runtime_text("mode.random"),
                    "single" => crate::manage_localization::runtime_text("mode.single"),
                    "playlist" => crate::manage_localization::runtime_text("mode.playlist"),
                    _ => display_mode.clone(),
                }
            )
            .width(
                CONTROL_WIDTH
            )
            .show_ui(
                ui,
                |ui| {
                    for (choice, label_key) in [
                        ("ordered", "mode.ordered"),
                        ("random", "mode.random"),
                        ("single", "mode.single"),
                        ("playlist", "mode.playlist"),
                    ] {
                        ui.selectable_value(
                            display_mode,
                            choice.to_string(),
                            crate::manage_localization::runtime_text(label_key),
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
                    crate::manage_localization::runtime_text("target.policy")
                );


                let displayed_policy =
                    if single_policy_id.is_none()
                        || single_policy_name
                            .trim()
                            .is_empty()
                    {
                        crate::manage_localization::runtime_text("target.select_policy")
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
                                                crate::manage_localization::runtime_text_with_params(
                                                    "target.single_policy_selected",
                                                    &[
                                                        ("target", &target_name(target)),
                                                        ("name", &row.policy_key),
                                                    ],
                                                );

                                            ui.close();
                                        }
                                    }


                                    if eligible_count == 0 {
                                        ui.add_enabled(
                                            false,
                                            egui::Button::new(
                                                crate::manage_localization::runtime_text("target.no_eligible_policies")
                                            ),
                                        );
                                    }
                                },
                            );
                    },
                );


                ui.end_row();
            } else if display_mode.as_str()
                == "playlist"
            {
                ui.label(
                    crate::manage_localization::runtime_text("target.playlist")
                );


                let displayed_playlist =
                    if playlist_id.is_none()
                        || playlist_name
                            .trim()
                            .is_empty()
                    {
                        crate::manage_localization::runtime_text("target.select_playlist")
                            .to_string()
                    } else {
                        playlist_name
                            .clone()
                    };


                ui.menu_button(
                    displayed_playlist,
                    |ui| {
                        match crate::manage_playlists::list_playlists() {
                            Ok(playlists) => {
                                if playlists.is_empty() {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(
                                            crate::manage_localization::runtime_text("target.no_playlists")
                                        ),
                                    );
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height(
                                            320.0
                                        )
                                        .show(
                                            ui,
                                            |ui| {
                                                for playlist in playlists {
                                                    let response =
                                                        ui.selectable_label(
                                                            *playlist_id
                                                                == Some(playlist.playlist_id),
                                                            playlist.playlist_name
                                                                .as_str(),
                                                        );

                                                    if response.clicked() {
                                                        *playlist_id =
                                                            Some(
                                                                playlist.playlist_id
                                                            );

                                                        *playlist_name =
                                                            playlist.playlist_name
                                                                .clone();

                                                        *status_message =
                                                            crate::manage_localization::runtime_text_with_params(
                                                                "target.playlist_selected",
                                                                &[
                                                                    ("target", &target_name(target)),
                                                                    ("name", &playlist.playlist_name),
                                                                ],
                                                            );

                                                        ui.close();
                                                    }
                                                }
                                            },
                                        );
                                }
                            }

                            Err(error) => {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(
                                        crate::manage_localization::runtime_text("target.playlists_unavailable")
                                    ),
                                )
                                .on_hover_text(
                                    error
                                );
                            }
                        }
                    },
                );


                ui.end_row();


                ui.label(
                    crate::manage_localization::runtime_text("target.interval")
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
                            crate::manage_localization::runtime_text("unit.seconds_lower")
                        );
                    },
                );


                ui.end_row();
            } else {
                ui.label(
                    crate::manage_localization::runtime_text("target.interval")
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
                            crate::manage_localization::runtime_text("unit.seconds_lower")
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
                    crate::manage_localization::runtime_text("target.idle_timeout")
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
                                "seconds" => crate::manage_localization::runtime_text("unit.seconds"),
                                "minutes" => crate::manage_localization::runtime_text("unit.minutes"),
                                "hours" => crate::manage_localization::runtime_text("unit.hours"),
                                _ => crate::manage_localization::runtime_text("unit.seconds"),
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
                                        "unit.seconds",
                                    ),
                                    (
                                        "minutes",
                                        "unit.minutes",
                                    ),
                                    (
                                        "hours",
                                        "unit.hours",
                                    ),
                                ] {
                                    ui.selectable_value(
                                        idle_timeout_unit,
                                        value.to_string(),
                                        crate::manage_localization::runtime_text(label),
                                    );
                                }
                            },
                        );
                    },
                );

                ui.end_row();
            }


            ui.label(
                crate::manage_localization::runtime_text("target.animation_speed")
            );

            ui.add(
                egui::DragValue::new(animation_speed)
                    .speed(0.01)
                    .clamp_range(0.001..=100.0)
                    .suffix("x")
            );

            ui.end_row();


            ui.label(
                crate::manage_localization::runtime_text("target.texture")
            );

            egui::ComboBox::from_id_source(
                format!(
                    "nested_config_texture_{:?}",
                    target,
                )
            )
            .selected_text(
                if global_texture == "random" {
                    crate::manage_localization::runtime_text("common.random")
                } else {
                    global_texture.clone()
                }
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
                        crate::manage_localization::runtime_text("common.random"),
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
                                    crate::manage_localization::runtime_text("target.texture_catalog_unavailable")
                                ),
                            );

                            *status_message =
                                crate::manage_localization::runtime_text_with_params(
                                    "target.texture_choices_failed",
                                    &[("error", &error)],
                                );
                        }
                    }
                },
            );

            ui.end_row();


            ui.label(
                crate::manage_localization::runtime_text("target.palette")
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
                crate::manage_localization::runtime_text("target.texture_primitives")
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
            crate::manage_localization::runtime_text_with_params(
                "target.single_policy_required",
                &[("target", &target_name(target))],
            );
    }
}


fn draw_lyrics(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    ui.heading(
        crate::manage_localization::runtime_text("lyrics.heading")
    );

    ui.add_space(
        8.0
    );

    ui.checkbox(
        &mut configuration.lyrics_enabled,
        crate::manage_localization::runtime_text("lyrics.display"),
    )
    .on_hover_text(
        crate::manage_localization::runtime_text("lyrics.display_help")
    );
}


fn draw_rendering_placeholders(
    ui: &mut egui::Ui,
    configuration: &mut ControlConfiguration,
) {
    const CONTROL_WIDTH: f32 = 190.0;

    ui.heading(crate::manage_localization::runtime_text("rendering.heading"));
    ui.add_space(8.0);

    egui::Grid::new("rendering_defaults")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 6.0))
        .show(ui, |ui| {
            ui.label(crate::manage_localization::runtime_text("rendering.fps"));
            ui.add(egui::DragValue::new(&mut configuration.rendered_fps).clamp_range(16..=120));
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.anti_aliasing"));
            egui::ComboBox::from_id_source("rendering_default_aa")
                .selected_text(match configuration.anti_aliasing.as_str() {
                    "off" => crate::manage_localization::runtime_text("rendering.off"),
                    "fxaa" => crate::manage_localization::runtime_text("rendering.fxaa"),
                    _ => configuration.anti_aliasing.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [("off", "rendering.off"), ("fxaa", "rendering.fxaa")] {
                        ui.selectable_value(&mut configuration.anti_aliasing, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label("Dithering:");
            egui::ComboBox::from_id_source("rendering_default_dithering")
                .selected_text(match configuration.dithering.as_str() {
                    "off" => crate::manage_localization::runtime_text("rendering.off"),
                    "subtle" => crate::manage_localization::runtime_text("rendering.subtle"),
                    _ => configuration.dithering.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [("off", "rendering.off"), ("subtle", "rendering.subtle")] {
                        ui.selectable_value(&mut configuration.dithering, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.color_precision"));
            egui::ComboBox::from_id_source("rendering_default_precision")
                .selected_text(match configuration.color_precision.as_str() {
                    "auto" => crate::manage_localization::runtime_text("rendering.auto"),
                    "standard" => crate::manage_localization::runtime_text("rendering.standard"),
                    "high" => crate::manage_localization::runtime_text("rendering.high"),
                    _ => configuration.color_precision.clone(),
                })
                .width(CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for (choice, key) in [
                        ("auto", "rendering.auto"),
                        ("standard", "rendering.standard"),
                        ("high", "rendering.high"),
                    ] {
                        ui.selectable_value(&mut configuration.color_precision, choice.to_string(), crate::manage_localization::runtime_text(key));
                    }
                });
            ui.end_row();

            ui.label(crate::manage_localization::runtime_text("rendering.render_scale"));
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
            crate::manage_localization::runtime_text("common.random")
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

        if ui.selectable_label(random_selected, crate::manage_localization::runtime_text("common.random")).clicked() {
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
                                *status_message = crate::manage_localization::runtime_text_with_params(
                                    "target.palette_selected",
                                    &[
                                        ("target", &target_name(target)),
                                        ("name", &entry.description),
                                    ],
                                );
                                ui.close();
                            }
                        }
                    });
            }

            Err(error) => {
                ui.add_enabled(
                    false,
                    egui::Button::new(crate::manage_localization::runtime_text("target.palette_unavailable")),
                );
                *status_message = crate::manage_localization::runtime_text_with_params(
                    "target.palette_choices_failed",
                    &[("error", error)],
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
) -> String {
    match target {
        PolicyTarget::Screensaver => crate::manage_localization::runtime_text("target.screensaver"),
        PolicyTarget::Wallpaper => crate::manage_localization::runtime_text("target.wallpaper"),
        PolicyTarget::Unassigned => "unassigned".to_string(),
    }
}
