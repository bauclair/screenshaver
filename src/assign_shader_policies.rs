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


// Manage Shader Policies: stage four independent target-presence flags.
// Unassigned is exclusive with all rendering targets.
#[derive(Clone, Debug)]
pub struct ManagedAssignmentRow {
    pub shader_id: i64,
    pub filename: String,
    pub existing: [bool; 4],
    pub has_no_policies: bool,
    pub selected: [bool; 4],
    // Counts include cloned policies; presence flags alone are insufficient
    // to calculate how many records an unchecked target would delete.
    pub existing_counts: [usize; 4],
}

impl ManagedAssignmentRow {
    pub fn set_target(&mut self, index: usize, checked: bool) {
        if index >= 4 { return; }
        self.selected[index] = checked;
        if checked {
            if index == 3 {
                self.selected[..3].fill(false);
            } else {
                self.selected[3] = false;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AssignmentChangeCounts {
    pub added: [usize; 4],
    pub deleted: [usize; 4],
}

impl AssignmentChangeCounts {
    pub fn additions(self) -> usize { self.added.iter().sum() }
    pub fn deletions(self) -> usize { self.deleted.iter().sum() }
    pub fn has_changes(self) -> bool { self.additions() + self.deletions() > 0 }
}

pub fn assignment_change_counts(rows: &[ManagedAssignmentRow]) -> AssignmentChangeCounts {
    let mut counts = AssignmentChangeCounts::default();
    for row in rows {
        for index in 0..4 {
            if row.selected[index] && !row.existing[index] { counts.added[index] += 1; }
            if !row.selected[index] && row.existing[index] {
                counts.deleted[index] += row.existing_counts[index];
            }
        }
    }
    counts
}

pub fn load_managed_assignments() -> Result<Vec<ManagedAssignmentRow>, String> {
    let connection = crate::open_database::open()?;
    let managed_dir = crate::locate_paths::shader_dir().to_string_lossy().to_string();
    let mut statement = connection.prepare(
        "SELECT s.shader_id, s.filename, \
         (SELECT COUNT(*) FROM shader_policies p WHERE p.shader_id=s.shader_id AND p.policy_target='screensaver'), \
         (SELECT COUNT(*) FROM shader_policies p WHERE p.shader_id=s.shader_id AND p.policy_target='wallpaper'), \
         (SELECT COUNT(*) FROM shader_policies p WHERE p.shader_id=s.shader_id AND p.policy_target='windowshader'), \
         (SELECT COUNT(*) FROM shader_policies p WHERE p.shader_id=s.shader_id AND p.policy_target='unassigned') \
         FROM shaders s WHERE s.source_path=?1 AND s.file_status <> 'missing' \
         AND s.filename COLLATE NOCASE <> 'default.glsl' \
         ORDER BY s.filename COLLATE NOCASE, s.filename, s.shader_id"
    ).map_err(|e| e.to_string())?;
    let rows = statement.query_map([managed_dir], |row| {
        let existing_counts = [
            row.get::<_, i64>(2)? as usize, row.get::<_, i64>(3)? as usize,
            row.get::<_, i64>(4)? as usize, row.get::<_, i64>(5)? as usize,
        ];
        let existing = existing_counts.map(|count| count != 0);
        Ok(ManagedAssignmentRow {
            shader_id: row.get(0)?, filename: row.get(1)?, existing,
            has_no_policies: existing_counts.iter().all(|count| *count == 0),
            selected: existing, existing_counts,
        })
    }).map_err(|e| e.to_string())?;
    let mut assignments = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    assignments.sort_by(|a, b| {
        // Display groups: shaders without policies, shaders with an existing
        // Unassigned policy, then all other shaders. Within each group,
        // filenames remain case-insensitively alphabetical.
        let group = |row: &ManagedAssignmentRow| -> u8 {
            if row.has_no_policies { 0 }
            else if row.existing[3] { 1 }
            else { 2 }
        };
        group(a).cmp(&group(b))
            .then_with(|| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()))
            .then_with(|| a.filename.cmp(&b.filename))
            .then_with(|| a.shader_id.cmp(&b.shader_id))
    });
    Ok(assignments)
}

// Retained for existing callers, but fail closed: the former additions-only
// writer must never bypass the mandatory backup and confirmation workflow.
pub fn create_selected_assignments(_rows: &[ManagedAssignmentRow]) -> Result<usize, String> {
    Err("Policy changes require the new confirmation and verified full-backup workflow; no changes were applied.".to_string())
}


/// Phase 3B.2: transaction-scoped writer. This intentionally does not open,
/// commit, or back up a database. The eventual coordinator must hold the
/// database write reservation, create and verify its pre-change backup, then
/// call this function and commit the transaction. Never call it directly from UI.
#[allow(dead_code)]
pub(crate) fn apply_managed_assignments_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    rows: &[ManagedAssignmentRow],
) -> Result<AssignmentChangeCounts, String> {
    use std::collections::HashSet;

    let targets = ["screensaver", "wallpaper", "windowshader", "unassigned"];
    let mut seen = HashSet::new();
    let mut verified = Vec::with_capacity(rows.len());
    let managed_dir = crate::locate_paths::shader_dir().to_string_lossy().to_string();
    let mut actual = AssignmentChangeCounts::default();

    // Validate every staged row before modifying any policy. In particular,
    // reject stale snapshots and any attempt to touch the protected shader.
    for row in rows {
        if row.shader_id <= 0 || !seen.insert(row.shader_id) {
            return Err("Invalid or repeated shader ID in policy change set".into());
        }
        if row.selected[3] && row.selected[..3].iter().any(|value| *value) {
            return Err(format!("Conflicting Unassigned selection for {}", row.filename));
        }
        let (filename, source_path, file_status): (String, String, String) = transaction
            .query_row(
                "SELECT filename, source_path, file_status FROM shaders WHERE shader_id=?1",
                [row.shader_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| format!("Shader {} changed or disappeared: {}", row.shader_id, e))?;
        if filename != row.filename || source_path != managed_dir
            || file_status == "missing" || filename.eq_ignore_ascii_case("default.glsl")
        {
            return Err(format!("Shader {} is no longer eligible for policy changes", row.filename));
        }
        let mut current = [0usize; 4];
        for (i, target) in targets.iter().enumerate() {
            let count: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM shader_policies WHERE shader_id=?1 AND policy_target=?2",
                params![row.shader_id, target],
                |r| r.get(0),
            ).map_err(|e| e.to_string())?;
            current[i] = count as usize;
        }
        if current != row.existing_counts
            || current.map(|count| count > 0) != row.existing
        {
            return Err(format!("Policies for {} changed since the dialog opened; reload and review", filename));
        }
        for i in 0..4 {
            if row.selected[i] && current[i] == 0 { actual.added[i] += 1; }
            if !row.selected[i] && current[i] > 0 { actual.deleted[i] += current[i]; }
        }
        verified.push((row.shader_id, filename, row.selected, current));
    }

    // Apply deletes first to free policy names. Foreign keys remove playlist
    // memberships and null out references to deleted Single policies.
    for (shader_id, _, selected, current) in &verified {
        for (i, target) in targets.iter().enumerate() {
            if !selected[i] && current[i] > 0 {
                let removed = transaction.execute(
                    "DELETE FROM shader_policies WHERE shader_id=?1 AND policy_target=?2",
                    params![shader_id, target],
                ).map_err(|e| format!("Could not delete {} policies: {}", target, e))?;
                if removed != current[i] {
                    return Err("Policy deletion count changed unexpectedly".into());
                }
            }
        }
    }

    for (shader_id, filename, selected, current) in &verified {
        for (i, target) in targets.iter().enumerate() {
            if selected[i] && current[i] == 0 {
                insert_default_policy(
                    transaction,
                    &PolicylessShader { shader_id: *shader_id, filename: filename.clone() },
                    target,
                )?;
            }
        }
    }

    // Preserve the selected display mode. For Single, a deleted selection
    // falls back to the protected default rather than changing mode.
    for target in &targets[..3] {
        let missing_single: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM runtime_targets WHERE target=?1 AND display_mode='single' AND single_policy_id IS NULL",
            [target], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        if missing_single > 0 {
            let fallback: i64 = transaction.query_row(
                "SELECT p.policy_id FROM shader_policies p JOIN shaders s ON s.shader_id=p.shader_id WHERE p.policy_target=?1 AND s.filename='default.glsl' ORDER BY p.policy_id LIMIT 1",
                [target], |r| r.get(0),
            ).map_err(|e| format!("Protected {} default policy unavailable: {}", target, e))?;
            transaction.execute(
                "UPDATE runtime_targets SET single_policy_id=?1 WHERE target=?2 AND display_mode='single' AND single_policy_id IS NULL",
                params![fallback, target],
            ).map_err(|e| e.to_string())?;
        }
    }
    // Policy deletion can empty a selected playlist or invalidate a rotation.
    // Keep the repair in the same transaction as the policy changes.
    for repair in crate::manage_configuration::repair_invalid_runtime_modes_in_connection(transaction)? {
        crate::logger::warning(&crate::locate_paths::runtime_log_path(),
            &format!("[RUNTIME FALLBACK] {}", repair));
    }
    Ok(actual)
}


/// Phase 3B.3: coordinated full backup and atomic policy update.
///
/// The dedicated connection is opened before the backup. SQLite's
/// `data_version` changes when another connection commits; checking it after
/// acquiring the IMMEDIATE write reservation detects intervening commits.
/// On any mismatch, the operation fails closed and the user must review again.
/// The UI must not call this until its confirmation flow is wired and tested.
#[allow(dead_code)]
pub(crate) fn apply_managed_assignments_with_backup(
    rows: &[ManagedAssignmentRow],
) -> Result<(AssignmentChangeCounts, std::path::PathBuf), String> {
    if rows.is_empty() {
        return Err("No shader assignments were supplied".to_string());
    }
    let requested = assignment_change_counts(rows);
    if !requested.has_changes() {
        return Err("No policy changes were selected; no backup was created".to_string());
    }

    // Keep this connection alive across the backup. A data_version comparison
    // on different connections is not meaningful.
    let mut connection = crate::open_database::open()?;
    let version_before: i64 = connection
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .map_err(|error| format!("Unable to inspect database version: {}", error))?;

    // A verified full backup is mandatory and must not reset the scheduled
    // backup clock. No database write is attempted if it fails.
    let archive = crate::manage_backup::create_pre_policy_change_backup()?;

    // Reserve the writer slot before comparing versions and validating rows.
    // Any error after this point drops the transaction and rolls back.
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("Unable to reserve database for policy changes: {}", error))?;
    let version_after: i64 = transaction
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .map_err(|error| format!("Unable to recheck database version: {}", error))?;
    if version_before != version_after {
        return Err("The database changed during the mandatory backup. No policies were modified; reopen Manage Shader Policies and review the changes.".to_string());
    }

    let actual = apply_managed_assignments_in_transaction(&transaction, rows)?;
    if actual.added != requested.added || actual.deleted != requested.deleted {
        return Err("Policy change totals no longer match the confirmation. No changes were committed.".to_string());
    }
    transaction.commit()
        .map_err(|error| format!("Unable to commit policy changes: {}", error))?;
    Ok((actual, archive))
}
