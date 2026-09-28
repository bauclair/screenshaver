use rusqlite::{
    params,
    Connection,
};

use sdl2::messagebox::{
    show_message_box,
    show_simple_message_box,
    ButtonData,
    ClickedButton,
    MessageBoxButtonFlag,
    MessageBoxFlag,
};


const BUTTON_SCREENSAVERS: i32 = 1;
const BUTTON_WALLPAPERS: i32 = 2;
const BUTTON_BOTH: i32 = 3;
const BUTTON_UNASSIGNED: i32 = 4;
const BUTTON_CANCEL: i32 = 5;


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum PolicyAssignment {

    Screensavers,

    Wallpapers,

    ScreensaversAndWallpapers,

    Unassigned,
}


impl PolicyAssignment {

    pub fn name(
        self,
    ) -> String {

        match self {

            Self::Screensavers => {
                crate::manage_localization::runtime_text(
                    "assign_shader_policies.all_screensavers"
                )
            }

            Self::Wallpapers => {
                crate::manage_localization::runtime_text(
                    "assign_shader_policies.all_wallpapers"
                )
            }

            Self::ScreensaversAndWallpapers => {
                crate::manage_localization::runtime_text(
                    "assign_shader_policies.screensavers_and_wallpapers"
                )
            }

            Self::Unassigned => {
                crate::manage_localization::runtime_text(
                    "assign_shader_policies.all_unassigned"
                )
            }
        }
    }
}


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum AssignmentOutcome {

    NoPoliciesNeeded,

    Dismissed {
        shader_count: usize,
    },

    Created {
        shader_count: usize,
        policy_count: usize,
        assignment: PolicyAssignment,
    },
}


#[derive(Debug)]
struct PolicylessShader {

    shader_id:
        i64,

    filename:
        String,
}


pub fn offer_assignment_if_needed(
    connection: &mut Connection,
) -> Result<AssignmentOutcome, String> {

    let shaders =
        load_policyless_managed_shaders(
            connection
        )?;


    if shaders.is_empty() {

        return Ok(
            AssignmentOutcome::NoPoliciesNeeded
        );
    }


    let shader_count =
        shaders.len();


    let assignment =
        match show_assignment_dialog(
            shader_count
        )? {

            Some(assignment) => {
                assignment
            }

            None => {
                return Ok(
                    AssignmentOutcome::Dismissed {
                        shader_count,
                    }
                );
            }
        };


    let policy_count =
        create_policies(
            connection,
            &shaders,
            assignment,
        )?;


    show_completion_dialog(
        shader_count,
        policy_count,
        assignment,
    );


    Ok(
        AssignmentOutcome::Created {
            shader_count,
            policy_count,
            assignment,
        }
    )
}


fn load_policyless_managed_shaders(
    connection: &Connection,
) -> Result<Vec<PolicylessShader>, String> {

    let managed_source_path =
        crate::locate_paths::shader_dir()
            .to_string_lossy()
            .to_string();


    let mut statement =
        connection
            .prepare(
                "SELECT
                     s.shader_id,
                     s.filename
                 FROM shaders AS s
                 WHERE s.source_path = ?1
                   AND s.file_status <> 'missing'
                   AND NOT EXISTS (
                       SELECT 1
                       FROM shader_policies AS p
                       WHERE p.shader_id = s.shader_id
                   )
                 ORDER BY
                     s.filename COLLATE NOCASE,
                     s.filename,
                     s.shader_id"
            )
            .map_err(
                |error| {
                    crate::manage_localization::runtime_text_with_params(
                        "assign_shader_policies.error.prepare_query",
                        &[
                            ("error", &error.to_string()),
                        ],
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [
                    managed_source_path
                ],
                |row| {
                    Ok(
                        PolicylessShader {
                            shader_id:
                                row.get(
                                    0
                                )?,

                            filename:
                                row.get(
                                    1
                                )?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    crate::manage_localization::runtime_text_with_params(
                        "assign_shader_policies.error.query_shaders",
                        &[
                            ("error", &error.to_string()),
                        ],
                    )
                }
            )?;


    let mut shaders =
        Vec::new();


    for row in rows {

        shaders.push(
            row.map_err(
                |error| {
                    crate::manage_localization::runtime_text_with_params(
                        "assign_shader_policies.error.decode_row",
                        &[
                            ("error", &error.to_string()),
                        ],
                    )
                }
            )?
        );
    }


    Ok(
        shaders
    )
}


fn show_assignment_dialog(
    shader_count: usize,
) -> Result<Option<PolicyAssignment>, String> {

    let all_screensavers =
        PolicyAssignment::Screensavers.name();
    let all_wallpapers =
        PolicyAssignment::Wallpapers.name();
    let screensavers_and_wallpapers =
        PolicyAssignment::ScreensaversAndWallpapers.name();
    let all_unassigned =
        PolicyAssignment::Unassigned.name();
    let cancel =
        crate::manage_localization::runtime_text(
            "common.cancel"
        );

    let buttons = [
        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_SCREENSAVERS,
            text:
                &all_screensavers,
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_WALLPAPERS,
            text:
                &all_wallpapers,
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_BOTH,
            text:
                &screensavers_and_wallpapers,
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_UNASSIGNED,
            text:
                &all_unassigned,
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_CANCEL,
            text:
                &cancel,
        },
    ];


    let shader_count_text =
        shader_count.to_string();

    let message_key =
        if shader_count == 1 {
            "assign_shader_policies.assignment_message.singular"
        } else {
            "assign_shader_policies.assignment_message.plural"
        };

    let message =
        crate::manage_localization::runtime_text_with_params(
            message_key,
            &[
                ("count", &shader_count_text),
            ],
        );


    let clicked =
        show_message_box(
            MessageBoxFlag::INFORMATION,
            &buttons,
            &crate::manage_localization::runtime_text(
                "assign_shader_policies.assignment_title"
            ),
            &message,
            None::<&sdl2::video::Window>,
            None::<sdl2::messagebox::MessageBoxColorScheme>,
        )
        .map_err(
            |error| {
                crate::manage_localization::runtime_text_with_params(
                    "assign_shader_policies.error.display_dialog",
                    &[
                        ("error", &format!("{:?}", error)),
                    ],
                )
            }
        )?;


    match clicked {

        ClickedButton::CloseButton => {
            Ok(
                None
            )
        }

        ClickedButton::CustomButton(
            button
        ) => {

            match button.button_id {

                BUTTON_SCREENSAVERS => {
                    Ok(
                        Some(
                            PolicyAssignment::Screensavers
                        )
                    )
                }

                BUTTON_WALLPAPERS => {
                    Ok(
                        Some(
                            PolicyAssignment::Wallpapers
                        )
                    )
                }

                BUTTON_BOTH => {
                    Ok(
                        Some(
                            PolicyAssignment::ScreensaversAndWallpapers
                        )
                    )
                }

                BUTTON_UNASSIGNED => {
                    Ok(
                        Some(
                            PolicyAssignment::Unassigned
                        )
                    )
                }

                BUTTON_CANCEL => {
                    Ok(
                        None
                    )
                }

                other => {
                    Err(
                        crate::manage_localization::runtime_text_with_params(
                            "assign_shader_policies.error.unknown_button",
                            &[
                                ("button_id", &other.to_string()),
                            ],
                        )
                    )
                }
            }
        }
    }
}


fn create_policies(
    connection: &mut Connection,
    shaders: &[PolicylessShader],
    assignment: PolicyAssignment,
) -> Result<usize, String> {

    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    crate::manage_localization::runtime_text_with_params(
                        "assign_shader_policies.error.begin_transaction",
                        &[
                            ("error", &error.to_string()),
                        ],
                    )
                }
            )?;


    let mut created =
        0_usize;


    for shader in shaders {

        // Recheck inside the transaction.  The dialog may have remained open
        // while another Control Center operation changed the database.
        let existing_count: i64 =
            transaction
                .query_row(
                    "SELECT COUNT(*)
                     FROM shader_policies
                     WHERE shader_id = ?1",
                    [
                        shader.shader_id
                    ],
                    |row| {
                        row.get(
                            0
                        )
                    },
                )
                .map_err(
                    |error| {
                        crate::manage_localization::runtime_text_with_params(
                            "assign_shader_policies.error.recheck_policies",
                            &[
                                ("filename", shader.filename.as_str()),
                                ("error", &error.to_string()),
                            ],
                        )
                    }
                )?;


        if existing_count
            != 0
        {
            continue;
        }


        match assignment {

            PolicyAssignment::Screensavers => {

                insert_default_policy(
                    &transaction,
                    shader,
                    "screensaver",
                )?;

                created +=
                    1;
            }

            PolicyAssignment::Wallpapers => {

                insert_default_policy(
                    &transaction,
                    shader,
                    "wallpaper",
                )?;

                created +=
                    1;
            }

            PolicyAssignment::ScreensaversAndWallpapers => {

                insert_default_policy(
                    &transaction,
                    shader,
                    "screensaver",
                )?;

                insert_default_policy(
                    &transaction,
                    shader,
                    "wallpaper",
                )?;

                created +=
                    2;
            }

            PolicyAssignment::Unassigned => {

                insert_default_policy(
                    &transaction,
                    shader,
                    "unassigned",
                )?;

                created +=
                    1;
            }
        }
    }


    transaction
        .commit()
        .map_err(
            |error| {
                crate::manage_localization::runtime_text_with_params(
                    "assign_shader_policies.error.commit_transaction",
                    &[
                        ("error", &error.to_string()),
                    ],
                )
            }
        )?;


    Ok(
        created
    )
}


fn insert_default_policy(
    connection: &Connection,
    shader: &PolicylessShader,
    target: &str,
) -> Result<(), String> {

    let policy_name =
        available_policy_name(
            connection,
            &shader.filename,
            target,
        )?;


    let policy_name_key =
        policy_name
            .chars()
            .flat_map(
                |character| {
                    character.to_lowercase()
                }
            )
            .collect::<String>();


    connection
        .execute(
            "INSERT INTO shader_policies (
                 policy_created_at,
                 policy_modified_at,
                 policy_name,
                 policy_name_key,
                 shader_id,
                 policy_target
             )
             VALUES (
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 ?1,
                 ?2,
                 ?3,
                 ?4
             )",
            params![
                policy_name,
                policy_name_key,
                shader.shader_id,
                target,
            ],
        )
        .map_err(
            |error| {
                crate::manage_localization::runtime_text_with_params(
                    "assign_shader_policies.error.create_policy",
                    &[
                        ("target", target),
                        ("filename", shader.filename.as_str()),
                        ("error", &error.to_string()),
                    ],
                )
            }
        )?;


    Ok(())
}


fn generated_policy_name(
    filename: &str,
) -> String {

    let filename =
        filename.trim();

    std::path::Path::new(
        filename
    )
    .file_stem()
    .and_then(
        |value| value.to_str()
    )
    .filter(
        |value| !value.trim().is_empty()
    )
    .unwrap_or(
        filename
    )
    .trim()
    .chars()
    .take(128)
    .collect::<String>()
}


fn available_policy_name(
    connection: &Connection,
    filename: &str,
    target: &str,
) -> Result<String, String> {

    let base =
        generated_policy_name(
            filename
        );

    for ordinal in 1_u32..=10_000 {
        let candidate =
            if ordinal == 1 {
                base.clone()
            } else {
                let suffix =
                    format!(
                        " ({})",
                        ordinal,
                    );

                let stem_limit =
                    128_usize.saturating_sub(
                        suffix.chars().count()
                    );

                format!(
                    "{}{}",
                    base.chars()
                        .take(stem_limit)
                        .collect::<String>(),
                    suffix,
                )
            };

        let key =
            candidate
                .chars()
                .flat_map(
                    |character| character.to_lowercase()
                )
                .collect::<String>();

        let count: i64 =
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM shader_policies
                     WHERE policy_name_key = ?1
                       AND policy_target = ?2",
                    params![
                        key,
                        target,
                    ],
                    |row| row.get(0),
                )
                .map_err(
                    |error| {
                        crate::manage_localization::runtime_text_with_params(
                            "assign_shader_policies.error.validate_policy_name",
                            &[
                                ("policy_name", candidate.as_str()),
                                ("target", target),
                                ("error", &error.to_string()),
                            ],
                        )
                    }
                )?;

        if count == 0 {
            return Ok(
                candidate
            );
        }
    }

    Err(
        crate::manage_localization::runtime_text_with_params(
            "assign_shader_policies.error.generate_policy_name",
            &[
                ("filename", filename),
                ("target", target),
            ],
        )
    )
}


fn show_completion_dialog(
    shader_count: usize,
    policy_count: usize,
    assignment: PolicyAssignment,
) {

    let policy_count_text =
        policy_count.to_string();
    let shader_count_text =
        shader_count.to_string();
    let assignment_name =
        assignment.name();

    let message_key =
        match (
            policy_count == 1,
            shader_count == 1,
        ) {
            (true, true) => {
                "assign_shader_policies.completion_message.one_policy_one_shader"
            }
            (true, false) => {
                "assign_shader_policies.completion_message.one_policy_many_shaders"
            }
            (false, true) => {
                "assign_shader_policies.completion_message.many_policies_one_shader"
            }
            (false, false) => {
                "assign_shader_policies.completion_message.many_policies_many_shaders"
            }
        };

    let message =
        crate::manage_localization::runtime_text_with_params(
            message_key,
            &[
                ("policy_count", &policy_count_text),
                ("shader_count", &shader_count_text),
                ("assignment", &assignment_name),
            ],
        );


    let _ =
        show_simple_message_box(
            MessageBoxFlag::INFORMATION,
            &crate::manage_localization::runtime_text(
                "assign_shader_policies.completion_title"
            ),
            &message,
            None::<&sdl2::video::Window>,
        );
}
