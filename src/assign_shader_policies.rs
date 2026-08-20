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
    ) -> &'static str {

        match self {

            Self::Screensavers => {
                "All Screensavers"
            }

            Self::Wallpapers => {
                "All Wallpapers"
            }

            Self::ScreensaversAndWallpapers => {
                "Screensavers + Wallpapers"
            }

            Self::Unassigned => {
                "All Unassigned"
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
                    format!(
                        "Unable to prepare policy-less shader query: {}",
                        error,
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
                    format!(
                        "Unable to query policy-less shaders: {}",
                        error,
                    )
                }
            )?;


    let mut shaders =
        Vec::new();


    for row in rows {

        shaders.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode policy-less shader row: {}",
                        error,
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

    let buttons = [
        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_SCREENSAVERS,
            text:
                "All Screensavers",
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_WALLPAPERS,
            text:
                "All Wallpapers",
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_BOTH,
            text:
                "Screensavers + Wallpapers",
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_UNASSIGNED,
            text:
                "All Unassigned",
        },

        ButtonData {
            flags:
                MessageBoxButtonFlag::empty(),
            button_id:
                BUTTON_CANCEL,
            text:
                "Cancel",
        },
    ];


    let message =
        format!(
            "Screenshaver found {} shader{} in the managed shaders folder that do not yet have a policy.\n\nChoose how policies should be created for these shaders.\n\nUnassigned policies cannot be rendered until their Policy Target is changed to Screensaver or Wallpaper.",
            shader_count,
            if shader_count == 1 {
                ""
            } else {
                "s"
            },
        );


    let clicked =
        show_message_box(
            MessageBoxFlag::INFORMATION,
            &buttons,
            "Assign New Shader Policies",
            &message,
            None::<&sdl2::video::Window>,
            None::<sdl2::messagebox::MessageBoxColorScheme>,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to display new-policy assignment dialog: {:?}",
                    error,
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
                        format!(
                            "New-policy assignment dialog returned unknown button id {}",
                            other,
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
                    format!(
                        "Unable to begin new-policy assignment transaction: {}",
                        error,
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
                        format!(
                            "Unable to recheck policies for '{}': {}",
                            shader.filename,
                            error,
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
                format!(
                    "Unable to commit new-policy assignment transaction: {}",
                    error,
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
        generated_policy_name(
            &shader.filename,
            target,
        );


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
                 policy_name,
                 policy_name_key,
                 shader_id,
                 policy_target
             )
             VALUES (
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
                format!(
                    "Unable to create {} policy for '{}': {}",
                    target,
                    shader.filename,
                    error,
                )
            }
        )?;


    Ok(())
}


fn generated_policy_name(
    filename: &str,
    target: &str,
) -> String {

    let suffix =
        format!(
            " ({})",
            target,
        );


    let maximum_filename_characters =
        128_usize
            .saturating_sub(
                suffix
                    .chars()
                    .count()
            );


    let trimmed_filename =
        filename
            .trim()
            .chars()
            .take(
                maximum_filename_characters
            )
            .collect::<String>();


    format!(
        "{}{}",
        trimmed_filename,
        suffix,
    )
}


fn show_completion_dialog(
    shader_count: usize,
    policy_count: usize,
    assignment: PolicyAssignment,
) {

    let message =
        format!(
            "Screenshaver created {} shader polic{} for {} shader{} using \"{}\".\n\nYou can review or change shader policies at any time by running:\n\nscreenshaver --control\n\nThis opens the Screenshaver Control Center.",
            policy_count,
            if policy_count == 1 {
                "y"
            } else {
                "ies"
            },
            shader_count,
            if shader_count == 1 {
                ""
            } else {
                "s"
            },
            assignment.name(),
        );


    let _ =
        show_simple_message_box(
            MessageBoxFlag::INFORMATION,
            "Shader Policies Created",
            &message,
            None::<&sdl2::video::Window>,
        );
}
