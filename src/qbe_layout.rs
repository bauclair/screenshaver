//! Query By Example strip layout for the Policy List.
//!
//! The visual geometry in this module is the approved two-line QBE layout.
//! QBE semantics, blank-state rules, contextual operators, and value kinds
//! come from parse_qbe.rs. SQL execution is intentionally not connected yet.

pub type QbeLayoutState =
    crate::parse_qbe::QbeState;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeStripAction {
    None,
    Query,
    Clear,
}


pub fn draw_qbe_strip(
    ui: &mut egui::Ui,
    scale: f32,
    state: &mut QbeLayoutState,
) -> QbeStripAction {
    let mut action =
        QbeStripAction::None;


    let old_spacing =
        ui.spacing().item_spacing;

    ui.spacing_mut().item_spacing.x =
        4.0 * scale;


    state.normalize();


    let lookup_values =
        crate::query_database::load_qbe_lookup_values()
            .unwrap_or_default();


    let query_ready =
        crate::parse_qbe::validate(
            state
        )
        .is_ok();


    // --------------------------------------------------------
    // Row 1:
    // Query | Clear | Item 1 | Operator 1 | Value 1
    // --------------------------------------------------------
    ui.horizontal(
        |ui| {
            let query_clicked =
                ui.add_enabled(
                    query_ready,
                    egui::Button::new(
                        "Query"
                    )
                    .min_size(
                        egui::vec2(
                            46.0 * scale,
                            22.0 * scale,
                        )
                    ),
                )
                .clicked();


            if query_clicked {
                action =
                    QbeStripAction::Query;
            }


            if ui.add(
                egui::Button::new(
                    "Clear"
                )
                .min_size(
                    egui::vec2(
                        46.0 * scale,
                        22.0 * scale,
                    )
                ),
            )
            .clicked()
            {
                state.clear();

                action =
                    QbeStripAction::Clear;
            }


            ui.add_space(
                4.0 * scale
            );


            draw_field_combo(
                ui,
                "qbe_item_1",
                130.0 * scale,
                true,
                &mut state.first,
            );


            draw_operator_combo(
                ui,
                "qbe_operator_1",
                72.0 * scale,
                &mut state.first,
            );


            draw_value_control(
                ui,
                "qbe_value_1",
                150.0 * scale,
                &mut state.first,
                &lookup_values,
            );
        },
    );


    ui.add_space(
        4.0 * scale
    );


    // --------------------------------------------------------
    // Row 2:
    // AND/OR | Item 2 | Operator 2 | Value 2
    // --------------------------------------------------------
    ui.horizontal(
        |ui| {
            let clause_1_complete =
                state.first.is_complete();


            draw_conditional_combo(
                ui,
                "qbe_conditional",
                72.0 * scale,
                clause_1_complete,
                &mut state.conditional,
            );


            if state.conditional.is_none() {
                state.second.clear();
            }


            draw_field_combo(
                ui,
                "qbe_item_2",
                130.0 * scale,
                state.conditional.is_some(),
                &mut state.second,
            );


            draw_operator_combo(
                ui,
                "qbe_operator_2",
                72.0 * scale,
                &mut state.second,
            );


            draw_value_control(
                ui,
                "qbe_value_2",
                150.0 * scale,
                &mut state.second,
                &lookup_values,
            );
        },
    );


    ui.spacing_mut().item_spacing =
        old_spacing;


    action
}


fn draw_field_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    enabled: bool,
    clause: &mut crate::parse_qbe::QbeClause,
) {
    let previous_field =
        clause.field;


    ui.add_enabled_ui(
        enabled,
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                clause.field
                    .map(
                        |field| field.label()
                    )
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for field in
                        crate::parse_qbe::QbeField::ALL
                    {
                        ui.selectable_value(
                            &mut clause.field,
                            Some(*field),
                            field.label(),
                        );
                    }
                },
            );
        },
    );


    if clause.field != previous_field {
        clause.operator =
            None;

        clause.value.clear();
    }


    clause.normalize_after_field_change();
}


fn draw_operator_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    clause: &mut crate::parse_qbe::QbeClause,
) {
    let previous_operator =
        clause.operator;


    let Some(field) =
        clause.field
    else {
        clause.operator =
            None;

        clause.value.clear();


        ui.add_enabled_ui(
            false,
            |ui| {
                egui::ComboBox::from_id_source(
                    id
                )
                .selected_text("")
                .width(width)
                .show_ui(
                    ui,
                    |_ui| {}
                );
            },
        );

        return;
    };


    ui.add_enabled_ui(
        true,
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                clause.operator
                    .map(
                        |operator| operator.label()
                    )
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for operator in
                        crate::parse_qbe::operators_for(
                            field
                        )
                    {
                        ui.selectable_value(
                            &mut clause.operator,
                            Some(*operator),
                            operator.label(),
                        );
                    }
                },
            );
        },
    );


    if clause.operator != previous_operator {
        clause.value.clear();
    }


    clause.normalize_after_operator_change();
}


fn draw_conditional_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    enabled: bool,
    selected: &mut Option<crate::parse_qbe::QbeConditional>,
) {
    if !enabled {
        *selected =
            None;
    }


    ui.add_enabled_ui(
        enabled,
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                selected
                    .map(
                        |conditional| conditional.label()
                    )
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for conditional in
                        crate::parse_qbe::QbeConditional::ALL
                    {
                        ui.selectable_value(
                            selected,
                            Some(*conditional),
                            conditional.label(),
                        );
                    }
                },
            );
        },
    );
}


fn draw_value_control(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    clause: &mut crate::parse_qbe::QbeClause,
    lookup_values: &crate::query_database::QbeLookupValues,
) {
    let (
        Some(field),
        Some(operator),
    ) = (
        clause.field,
        clause.operator,
    )
    else {
        clause.value.clear();


        ui.add_enabled_ui(
            false,
            |ui| {
                ui.add_sized(
                    [
                        width,
                        22.0,
                    ],
                    egui::TextEdit::singleline(
                        &mut clause.value
                    )
                    .id_source(id),
                );
            },
        );

        return;
    };


    match crate::parse_qbe::value_kind_for(
        field,
        operator,
    ) {
        crate::parse_qbe::QbeValueKind::Boolean => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "true",
                    "false",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::ShaderType => {
            draw_string_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &lookup_values.shader_types,
            );
        }


        crate::parse_qbe::QbeValueKind::PolicyTarget => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "Screensaver",
                    "Wallpaper",
                    "Unassigned",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::TextureName => {
            draw_string_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &lookup_values.texture_names,
            );
        }


        crate::parse_qbe::QbeValueKind::PaletteName => {
            draw_palette_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &lookup_values.palette_choices,
            );
        }


        crate::parse_qbe::QbeValueKind::Status => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "OK",
                    "Rejected",
                    "Missing",
                    "Unreadable",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::AntiAliasing => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "Off",
                    "FXAA",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::Dithering => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "Off",
                    "Subtle",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::ColorPrecision => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "Automatic",
                    "High Precision",
                    "Standard Precision",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::BloomMode => {
            draw_value_combo(
                ui,
                id,
                width,
                &mut clause.value,
                &[
                    "Off",
                    "Highlight",
                    "Audio",
                ],
            );
        }


        crate::parse_qbe::QbeValueKind::Text
        | crate::parse_qbe::QbeValueKind::Integer
        | crate::parse_qbe::QbeValueKind::Decimal => {
            ui.add_sized(
                [
                    width,
                    22.0,
                ],
                egui::TextEdit::singleline(
                    &mut clause.value
                )
                .id_source(id),
            );
        }
    }
}


fn draw_value_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    value: &mut String,
    choices: &[&str],
) {
    egui::ComboBox::from_id_source(
        id
    )
    .selected_text(
        value.as_str()
    )
    .width(width)
    .show_ui(
        ui,
        |ui| {
            for choice in choices {
                ui.selectable_value(
                    value,
                    (*choice).to_string(),
                    *choice,
                );
            }
        },
    );
}


fn draw_string_value_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    value: &mut String,
    choices: &[String],
) {
    egui::ComboBox::from_id_source(
        id
    )
    .selected_text(
        value.as_str()
    )
    .width(width)
    .show_ui(
        ui,
        |ui| {
            for choice in choices {
                ui.selectable_value(
                    value,
                    choice.clone(),
                    choice.as_str(),
                );
            }
        },
    );
}


fn draw_palette_value_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    value: &mut String,
    choices: &[crate::query_database::QbePaletteChoice],
) {
    egui::ComboBox::from_id_source(
        id
    )
    .selected_text(
        value.as_str()
    )
    .width(width)
    .show_ui(
        ui,
        |ui| {
            ui.selectable_value(
                value,
                "random".to_string(),
                "random",
            );


            for choice in choices {
                let label =
                    if choice.description.trim().is_empty() {
                        choice.color_hex.clone()
                    } else {
                        format!(
                            "{} ({})",
                            choice.description,
                            choice.color_hex,
                        )
                    };


                ui.selectable_value(
                    value,
                    choice.color_hex.clone(),
                    label,
                );
            }
        },
    );
}

