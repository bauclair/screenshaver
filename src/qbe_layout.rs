//! Query By Example strip layout for the Policy List.
//!
//! This first implementation is deliberately a UI/layout checkpoint.
//! It owns only transient mockup state and contextual control behavior.
//! SQL parsing and database execution will be supplied later by
//! parse_qbe.rs and query_database.rs.

#[derive(Clone, Debug, Default)]
pub struct QbeLayoutState {
    pub item_1: Option<String>,
    pub operator_1: Option<String>,
    pub value_1: String,
    pub conditional: Option<String>,
    pub item_2: Option<String>,
    pub operator_2: Option<String>,
    pub value_2: String,
}


const ITEMS: &[&str] = &[
    "Policy Name",
    "Shader Filename",
    "Shader Type",
    "Policy Target",
    "Status",
    "Texture",
    "Palette",
    "Rendered FPS",
    "Animation Speed",
    "Render Scale",
    "Anti-Aliasing",
    "Dithering",
    "Color Precision",
    "Bloom Mode",
];


pub fn draw_qbe_strip(
    ui: &mut egui::Ui,
    scale: f32,
    state: &mut QbeLayoutState,
) {
    let old_spacing =
        ui.spacing().item_spacing;

    ui.spacing_mut().item_spacing.x =
        4.0 * scale;

    let clause_1_complete =
        clause_complete(
            &state.item_1,
            &state.operator_1,
            &state.value_1,
        );

    let clause_2_complete =
        clause_complete(
            &state.item_2,
            &state.operator_2,
            &state.value_2,
        );

    let query_ready =
        clause_1_complete
            && (
                state.conditional.is_none()
                    || clause_2_complete
            );

    // --------------------------------------------------------
    // Row 1:
    // Query | Clear | Item 1 | Operator 1 | Value 1
    // --------------------------------------------------------
    ui.horizontal(
        |ui| {
            let _query_clicked =
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

            draw_item_combo(
                ui,
                "qbe_item_1",
                130.0 * scale,
                &mut state.item_1,
            );

            if state.item_1.is_none() {
                state.operator_1 = None;
                state.value_1.clear();
            }

            draw_operator_combo(
                ui,
                "qbe_operator_1",
                72.0 * scale,
                state.item_1.as_deref(),
                &mut state.operator_1,
            );

            if !operator_valid(
                state.item_1.as_deref(),
                state.operator_1.as_deref(),
            ) {
                state.operator_1 = None;
                state.value_1.clear();
            }

            draw_value_control(
                ui,
                "qbe_value_1",
                150.0 * scale,
                state.item_1.as_deref(),
                state.operator_1.as_deref(),
                &mut state.value_1,
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
                *state =
                    QbeLayoutState::default();
            }

            ui.add_space(
                4.0 * scale
            );

            draw_conditional_combo(
                ui,
                "qbe_conditional",
                72.0 * scale,
                clause_1_complete,
                &mut state.conditional,
            );

            if state.conditional.is_none() {
                state.item_2 = None;
                state.operator_2 = None;
                state.value_2.clear();
            }

            draw_item_combo_enabled(
                ui,
                "qbe_item_2",
                130.0 * scale,
                state.conditional.is_some(),
                &mut state.item_2,
            );

            if state.item_2.is_none() {
                state.operator_2 = None;
                state.value_2.clear();
            }

            draw_operator_combo(
                ui,
                "qbe_operator_2",
                72.0 * scale,
                if state.conditional.is_some() {
                    state.item_2.as_deref()
                } else {
                    None
                },
                &mut state.operator_2,
            );

            if !operator_valid(
                state.item_2.as_deref(),
                state.operator_2.as_deref(),
            ) {
                state.operator_2 = None;
                state.value_2.clear();
            }

            draw_value_control(
                ui,
                "qbe_value_2",
                150.0 * scale,
                if state.conditional.is_some() {
                    state.item_2.as_deref()
                } else {
                    None
                },
                state.operator_2.as_deref(),
                &mut state.value_2,
            );
        },
    );

    ui.spacing_mut().item_spacing =
        old_spacing;
}


fn clause_complete(
    item: &Option<String>,
    operator: &Option<String>,
    value: &str,
) -> bool {
    item.is_some()
        && operator.is_some()
        && !value.trim().is_empty()
}


fn draw_item_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    selected: &mut Option<String>,
) {
    draw_item_combo_enabled(
        ui,
        id,
        width,
        true,
        selected,
    );
}


fn draw_item_combo_enabled(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    enabled: bool,
    selected: &mut Option<String>,
) {
    ui.add_enabled_ui(
        enabled,
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                selected.as_deref()
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for item in ITEMS {
                        ui.selectable_value(
                            selected,
                            Some(
                                (*item).to_string()
                            ),
                            *item,
                        );
                    }
                },
            );
        },
    );
}


fn draw_operator_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    item: Option<&str>,
    selected: &mut Option<String>,
) {
    let operators =
        operators_for(item);

    ui.add_enabled_ui(
        item.is_some(),
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                selected.as_deref()
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for operator in operators {
                        ui.selectable_value(
                            selected,
                            Some(
                                operator.to_string()
                            ),
                            *operator,
                        );
                    }
                },
            );
        },
    );
}


fn draw_conditional_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    width: f32,
    enabled: bool,
    selected: &mut Option<String>,
) {
    ui.add_enabled_ui(
        enabled,
        |ui| {
            egui::ComboBox::from_id_source(
                id
            )
            .selected_text(
                selected.as_deref()
                    .unwrap_or("")
            )
            .width(width)
            .show_ui(
                ui,
                |ui| {
                    for conditional in [
                        "AND",
                        "OR",
                    ] {
                        ui.selectable_value(
                            selected,
                            Some(
                                conditional.to_string()
                            ),
                            conditional,
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
    item: Option<&str>,
    operator: Option<&str>,
    value: &mut String,
) {
    let enabled =
        item.is_some()
            && operator.is_some();

    ui.add_enabled_ui(
        enabled,
        |ui| {
            match value_kind(
                item,
                operator,
            ) {
                MockValueKind::Boolean => {
                    draw_value_combo(
                        ui,
                        id,
                        width,
                        value,
                        &[
                            "true",
                            "false",
                        ],
                    );
                }

                MockValueKind::ShaderType => {
                    draw_value_combo(
                        ui,
                        id,
                        width,
                        value,
                        &[
                            "NativeGLSL",
                            "ISF",
                            "ShaderToy",
                        ],
                    );
                }

                MockValueKind::PolicyTarget => {
                    draw_value_combo(
                        ui,
                        id,
                        width,
                        value,
                        &[
                            "Screensaver",
                            "Wallpaper",
                            "Unassigned",
                        ],
                    );
                }

                MockValueKind::TextureName => {
                    // Temporary visual choices only. query_database.rs will
                    // provide the live database-backed texture list later.
                    draw_value_combo(
                        ui,
                        id,
                        width,
                        value,
                        &[
                            "Bricks",
                            "Marble",
                            "Skulls",
                        ],
                    );
                }

                MockValueKind::Status => {
                    draw_value_combo(
                        ui,
                        id,
                        width,
                        value,
                        &[
                            "OK",
                            "Rejected",
                            "Missing",
                            "Unreadable",
                        ],
                    );
                }

                MockValueKind::Text
                | MockValueKind::Numeric => {
                    ui.add_sized(
                        [
                            width,
                            22.0,
                        ],
                        egui::TextEdit::singleline(
                            value
                        )
                        .id_source(id),
                    );
                }
            }
        },
    );
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


fn operators_for(
    item: Option<&str>,
) -> &'static [&'static str] {
    match item {
        Some(
            "Rendered FPS"
            | "Animation Speed"
            | "Render Scale"
        ) => {
            &[
                "eq",
                "ne",
                "lt",
                "le",
                "gt",
                "ge",
            ]
        }

        Some(
            "Shader Type"
            | "Policy Target"
            | "Status"
            | "Anti-Aliasing"
            | "Dithering"
            | "Color Precision"
            | "Bloom Mode"
        ) => {
            &[
                "eq",
                "ne",
            ]
        }

        Some(
            "Texture"
            | "Palette"
        ) => {
            &[
                "is",
                "eq",
                "ne",
                "like",
                "not like",
            ]
        }

        Some(_) => {
            &[
                "eq",
                "ne",
                "like",
                "not like",
            ]
        }

        None => {
            &[]
        }
    }
}


fn operator_valid(
    item: Option<&str>,
    operator: Option<&str>,
) -> bool {
    let Some(operator) =
        operator
    else {
        return true;
    };

    operators_for(item)
        .contains(
            &operator
        )
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockValueKind {
    Boolean,
    Text,
    Numeric,
    ShaderType,
    PolicyTarget,
    TextureName,
    Status,
}


fn value_kind(
    item: Option<&str>,
    operator: Option<&str>,
) -> MockValueKind {
    match (
        item,
        operator,
    ) {
        (
            Some(
                "Texture"
                | "Palette"
            ),
            Some("is"),
        ) => {
            MockValueKind::Boolean
        }

        (
            Some("Texture"),
            _,
        ) => {
            MockValueKind::TextureName
        }

        (
            Some("Shader Type"),
            _,
        ) => {
            MockValueKind::ShaderType
        }

        (
            Some("Policy Target"),
            _,
        ) => {
            MockValueKind::PolicyTarget
        }

        (
            Some("Status"),
            _,
        ) => {
            MockValueKind::Status
        }

        (
            Some(
                "Rendered FPS"
                | "Animation Speed"
                | "Render Scale"
            ),
            _,
        ) => {
            MockValueKind::Numeric
        }

        _ => {
            MockValueKind::Text
        }
    }
}
