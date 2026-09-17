use std::collections::HashSet;

use rusqlite::{
    params,
    Connection,
    OptionalExtension,
};


#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct PlaylistSummary {

    pub playlist_id:
        i64,

    pub playlist_name:
        String,

    pub description:
        Option<String>,

    pub member_count:
        i64,
}


#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct PlaylistMember {

    pub playlist_id:
        i64,

    pub policy_id:
        i64,

    pub position:
        i64,

    pub policy_name:
        String,

    pub policy_target:
        String,

    pub shader_filename:
        String,
}


#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
)]
pub struct AddPoliciesResult {

    pub added:
        usize,

    pub skipped_existing:
        usize,
}


pub fn create_playlist(
    playlist_name: &str,
    description: Option<&str>,
) -> Result<i64, String> {

    let playlist_name =
        normalized_playlist_name(
            playlist_name
        )?;

    let playlist_name_key =
        playlist_name_key(
            &playlist_name
        )?;

    let description =
        normalized_description(
            description
        );


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while creating playlist '{}': {}",
                        playlist_name,
                        error,
                    )
                }
            )?;


    ensure_playlist_name_available(
        &connection,
        &playlist_name_key,
        None,
    )?;


    connection
        .execute(
            "INSERT INTO playlists (
                 playlist_name,
                 playlist_name_key,
                 description
             )
             VALUES (?1, ?2, ?3)",
            params![
                playlist_name,
                playlist_name_key,
                description,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to create playlist '{}': {}",
                    playlist_name,
                    error,
                )
            }
        )?;


    Ok(
        connection.last_insert_rowid()
    )
}


pub fn rename_playlist(
    playlist_id: i64,
    new_playlist_name: &str,
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let playlist_name =
        normalized_playlist_name(
            new_playlist_name
        )?;

    let playlist_name_key =
        playlist_name_key(
            &playlist_name
        )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while renaming playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &connection,
        playlist_id,
    )?;


    ensure_playlist_name_available(
        &connection,
        &playlist_name_key,
        Some(playlist_id),
    )?;


    let changed =
        connection
            .execute(
                "UPDATE playlists
                 SET playlist_name = ?1,
                     playlist_name_key = ?2
                 WHERE playlist_id = ?3",
                params![
                    playlist_name,
                    playlist_name_key,
                    playlist_id,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to rename playlist ID {} as '{}': {}",
                        playlist_id,
                        playlist_name,
                        error,
                    )
                }
            )?;


    expect_one_changed(
        changed,
        &format!(
            "rename playlist ID {}",
            playlist_id,
        ),
    )
}


pub fn update_playlist_description(
    playlist_id: i64,
    description: Option<&str>,
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let description =
        normalized_description(
            description
        );


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while updating description for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let changed =
        connection
            .execute(
                "UPDATE playlists
                 SET description = ?1
                 WHERE playlist_id = ?2",
                params![
                    description,
                    playlist_id,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to update description for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    expect_one_changed(
        changed,
        &format!(
            "update description for playlist ID {}",
            playlist_id,
        ),
    )
}


pub fn delete_playlist(
    playlist_id: i64,
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while deleting playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let playlist_name =
        playlist_name_for_id(
            &connection,
            playlist_id,
        )?;


    let deleted =
        connection
            .execute(
                "DELETE FROM playlists
                 WHERE playlist_id = ?1",
                [playlist_id],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to delete playlist ID {} ('{}'): {}",
                        playlist_id,
                        playlist_name,
                        error,
                    )
                }
            )?;


    expect_one_changed(
        deleted,
        &format!(
            "delete playlist ID {} ('{}')",
            playlist_id,
            playlist_name,
        ),
    )
}


pub fn list_playlists(
) -> Result<Vec<PlaylistSummary>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while listing playlists: {}",
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     pl.playlist_id,
                     pl.playlist_name,
                     pl.description,
                     COUNT(pm.policy_id)
                 FROM playlists AS pl
                 LEFT JOIN playlist_members AS pm
                   ON pm.playlist_id = pl.playlist_id
                 GROUP BY
                     pl.playlist_id,
                     pl.playlist_name,
                     pl.playlist_name_key,
                     pl.description
                 ORDER BY
                     pl.playlist_name_key,
                     pl.playlist_name,
                     pl.playlist_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare playlist-list query: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| {
                    Ok(
                        PlaylistSummary {
                            playlist_id:
                                row.get(0)?,

                            playlist_name:
                                row.get(1)?,

                            description:
                                row.get(2)?,

                            member_count:
                                row.get(3)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query playlists: {}",
                        error,
                    )
                }
            )?;


    let mut playlists =
        Vec::new();


    for row in rows {
        playlists.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode playlist row: {}",
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        playlists
    )
}


pub fn playlist_members(
    playlist_id: i64,
) -> Result<Vec<PlaylistMember>, String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while loading members of playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &connection,
        playlist_id,
    )?;


    playlist_members_in_connection(
        &connection,
        playlist_id,
    )
}


/// Returns the members of a playlist that are eligible for the requested
/// runtime target, preserving the playlist's canonical persistent order.
///
/// A playlist may contain policies for multiple targets. Runtime selection
/// filters that shared ordering rather than maintaining a second target-specific
/// order. Unassigned policies are therefore excluded from both runtime targets.
pub fn playlist_render_policies(
    playlist_id: i64,
    target: &str,
) -> Result<Vec<PlaylistMember>, String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;

    let target =
        normalized_runtime_target(
            target
        )?;

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while resolving playlist ID {} for target '{}': {}",
                        playlist_id,
                        target,
                        error,
                    )
                }
            )?;

    ensure_playlist_exists(
        &connection,
        playlist_id,
    )?;

    let mut statement =
        connection
            .prepare(
                "SELECT
                     pm.playlist_id,
                     pm.policy_id,
                     pm.position,
                     p.policy_name,
                     p.policy_target,
                     s.filename
                 FROM playlist_members AS pm
                 JOIN shader_policies AS p
                   ON p.policy_id = pm.policy_id
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE pm.playlist_id = ?1
                   AND p.policy_target = ?2
                 ORDER BY
                     pm.position,
                     pm.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare runtime playlist query for playlist ID {} target '{}': {}",
                        playlist_id,
                        target,
                        error,
                    )
                }
            )?;

    let rows =
        statement
            .query_map(
                params![
                    playlist_id,
                    target,
                ],
                |row| {
                    Ok(
                        PlaylistMember {
                            playlist_id: row.get(0)?,
                            policy_id: row.get(1)?,
                            position: row.get(2)?,
                            policy_name: row.get(3)?,
                            policy_target: row.get(4)?,
                            shader_filename: row.get(5)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to resolve playlist ID {} for runtime target '{}': {}",
                        playlist_id,
                        target,
                        error,
                    )
                }
            )?;

    let mut members = Vec::new();

    for row in rows {
        members.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode runtime member of playlist ID {} target '{}': {}",
                        playlist_id,
                        target,
                        error,
                    )
                }
            )?
        );
    }

    Ok(members)
}


pub fn playlists_for_policy(
    policy_id: i64,
) -> Result<Vec<PlaylistSummary>, String> {

    require_positive_id(
        "Policy",
        policy_id,
    )?;


    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while finding playlists for policy ID {}: {}",
                        policy_id,
                        error,
                    )
                }
            )?;


    ensure_policy_exists(
        &connection,
        policy_id,
    )?;


    let mut statement =
        connection
            .prepare(
                "SELECT
                     pl.playlist_id,
                     pl.playlist_name,
                     pl.description,
                     (
                         SELECT COUNT(*)
                         FROM playlist_members AS all_members
                         WHERE all_members.playlist_id = pl.playlist_id
                     ) AS member_count
                 FROM playlist_members AS pm
                 JOIN playlists AS pl
                   ON pl.playlist_id = pm.playlist_id
                 WHERE pm.policy_id = ?1
                 ORDER BY
                     pl.playlist_name_key,
                     pl.playlist_name,
                     pl.playlist_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare reverse playlist lookup for policy ID {}: {}",
                        policy_id,
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [policy_id],
                |row| {
                    Ok(
                        PlaylistSummary {
                            playlist_id:
                                row.get(0)?,

                            playlist_name:
                                row.get(1)?,

                            description:
                                row.get(2)?,

                            member_count:
                                row.get(3)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query playlists for policy ID {}: {}",
                        policy_id,
                        error,
                    )
                }
            )?;


    let mut playlists =
        Vec::new();


    for row in rows {
        playlists.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode playlist lookup row for policy ID {}: {}",
                        policy_id,
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        playlists
    )
}


pub fn add_policy(
    playlist_id: i64,
    policy_id: i64,
) -> Result<bool, String> {

    let result =
        add_policies(
            playlist_id,
            &[policy_id],
        )?;


    Ok(
        result.added == 1
    )
}


pub fn add_policies(
    playlist_id: i64,
    policy_ids: &[i64],
) -> Result<AddPoliciesResult, String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    if policy_ids.is_empty() {
        return Ok(
            AddPoliciesResult::default()
        );
    }


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while adding policies to playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin playlist-add transaction for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &transaction,
        playlist_id,
    )?;


    let mut next_position: i64 =
        transaction
            .query_row(
                "SELECT COALESCE(MAX(position), 0) + 1
                 FROM playlist_members
                 WHERE playlist_id = ?1",
                [playlist_id],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to determine append position for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let mut result =
        AddPoliciesResult::default();

    let mut seen =
        HashSet::new();


    for &policy_id in policy_ids {

        require_positive_id(
            "Policy",
            policy_id,
        )?;


        if !seen.insert(
            policy_id
        ) {
            result.skipped_existing +=
                1;

            continue;
        }


        ensure_policy_exists(
            &transaction,
            policy_id,
        )?;


        let already_present: i64 =
            transaction
                .query_row(
                    "SELECT COUNT(*)
                     FROM playlist_members
                     WHERE playlist_id = ?1
                       AND policy_id = ?2",
                    params![
                        playlist_id,
                        policy_id,
                    ],
                    |row| {
                        row.get(0)
                    },
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to check policy ID {} membership in playlist ID {}: {}",
                            policy_id,
                            playlist_id,
                            error,
                        )
                    }
                )?;


        if already_present != 0 {
            result.skipped_existing +=
                1;

            continue;
        }


        transaction
            .execute(
                "INSERT INTO playlist_members (
                     playlist_id,
                     policy_id,
                     position
                 )
                 VALUES (?1, ?2, ?3)",
                params![
                    playlist_id,
                    policy_id,
                    next_position,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to add policy ID {} to playlist ID {} at position {}: {}",
                        policy_id,
                        playlist_id,
                        next_position,
                        error,
                    )
                }
            )?;


        next_position +=
            1;

        result.added +=
            1;
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit additions to playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )?;


    Ok(
        result
    )
}


pub fn remove_policy(
    playlist_id: i64,
    policy_id: i64,
) -> Result<bool, String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;

    require_positive_id(
        "Policy",
        policy_id,
    )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while removing policy ID {} from playlist ID {}: {}",
                        policy_id,
                        playlist_id,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin playlist-remove transaction for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &transaction,
        playlist_id,
    )?;


    let removed =
        transaction
            .execute(
                "DELETE FROM playlist_members
                 WHERE playlist_id = ?1
                   AND policy_id = ?2",
                params![
                    playlist_id,
                    policy_id,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to remove policy ID {} from playlist ID {}: {}",
                        policy_id,
                        playlist_id,
                        error,
                    )
                }
            )?;


    if removed > 1 {
        return Err(
            format!(
                "Removing policy ID {} from playlist ID {} unexpectedly removed {} rows",
                policy_id,
                playlist_id,
                removed,
            )
        );
    }


    if removed == 1 {
        compact_playlist_positions_in_connection(
            &transaction,
            playlist_id,
        )?;
    }


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit removal from playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )?;


    Ok(
        removed == 1
    )
}


pub fn replace_member_order(
    playlist_id: i64,
    ordered_policy_ids: &[i64],
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while reordering playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin reorder transaction for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &transaction,
        playlist_id,
    )?;


    let existing_ids =
        playlist_policy_ids_in_connection(
            &transaction,
            playlist_id,
        )?;


    validate_replacement_order(
        playlist_id,
        &existing_ids,
        ordered_policy_ids,
    )?;


    rewrite_positions_in_connection(
        &transaction,
        playlist_id,
        ordered_policy_ids,
    )?;


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit reordered playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )
}


pub fn move_policy(
    playlist_id: i64,
    policy_id: i64,
    new_position: usize,
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;

    require_positive_id(
        "Policy",
        policy_id,
    )?;


    let members =
        playlist_members(
            playlist_id
        )?;


    if members.is_empty() {
        return Err(
            format!(
                "Playlist ID {} has no members to reorder",
                playlist_id,
            )
        );
    }


    if new_position == 0
        || new_position > members.len()
    {
        return Err(
            format!(
                "Playlist position {} is outside the valid range 1..{}",
                new_position,
                members.len(),
            )
        );
    }


    let mut policy_ids =
        members
            .into_iter()
            .map(
                |member| {
                    member.policy_id
                }
            )
            .collect::<Vec<_>>();


    let Some(current_index) =
        policy_ids
            .iter()
            .position(
                |candidate| {
                    *candidate == policy_id
                }
            )
    else {
        return Err(
            format!(
                "Policy ID {} is not a member of playlist ID {}",
                policy_id,
                playlist_id,
            )
        );
    };


    let policy_id =
        policy_ids.remove(
            current_index
        );


    policy_ids.insert(
        new_position - 1,
        policy_id,
    );


    replace_member_order(
        playlist_id,
        &policy_ids,
    )
}


pub fn compact_playlist_positions(
    playlist_id: i64,
) -> Result<(), String> {

    require_positive_id(
        "Playlist",
        playlist_id,
    )?;


    let mut connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while compacting playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let transaction =
        connection
            .transaction()
            .map_err(
                |error| {
                    format!(
                        "Unable to begin compaction transaction for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    ensure_playlist_exists(
        &transaction,
        playlist_id,
    )?;


    compact_playlist_positions_in_connection(
        &transaction,
        playlist_id,
    )?;


    transaction
        .commit()
        .map_err(
            |error| {
                format!(
                    "Unable to commit position compaction for playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )
}


pub(crate) fn compact_playlist_positions_in_connection(
    connection: &Connection,
    playlist_id: i64,
) -> Result<(), String> {

    let policy_ids =
        playlist_policy_ids_in_connection(
            connection,
            playlist_id,
        )?;


    rewrite_positions_in_connection(
        connection,
        playlist_id,
        &policy_ids,
    )
}


fn playlist_members_in_connection(
    connection: &Connection,
    playlist_id: i64,
) -> Result<Vec<PlaylistMember>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT
                     pm.playlist_id,
                     pm.policy_id,
                     pm.position,
                     p.policy_name,
                     p.policy_target,
                     s.filename
                 FROM playlist_members AS pm
                 JOIN shader_policies AS p
                   ON p.policy_id = pm.policy_id
                 JOIN shaders AS s
                   ON s.shader_id = p.shader_id
                 WHERE pm.playlist_id = ?1
                 ORDER BY
                     pm.position,
                     pm.policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare member query for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [playlist_id],
                |row| {
                    Ok(
                        PlaylistMember {
                            playlist_id:
                                row.get(0)?,

                            policy_id:
                                row.get(1)?,

                            position:
                                row.get(2)?,

                            policy_name:
                                row.get(3)?,

                            policy_target:
                                row.get(4)?,

                            shader_filename:
                                row.get(5)?,
                        }
                    )
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query members of playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let mut members =
        Vec::new();


    for row in rows {
        members.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode member of playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        members
    )
}


fn playlist_policy_ids_in_connection(
    connection: &Connection,
    playlist_id: i64,
) -> Result<Vec<i64>, String> {

    let mut statement =
        connection
            .prepare(
                "SELECT policy_id
                 FROM playlist_members
                 WHERE playlist_id = ?1
                 ORDER BY
                     position,
                     policy_id"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare ordered policy-ID query for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [playlist_id],
                |row| {
                    row.get::<_, i64>(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query ordered policy IDs for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    let mut policy_ids =
        Vec::new();


    for row in rows {
        policy_ids.push(
            row.map_err(
                |error| {
                    format!(
                        "Unable to decode ordered policy ID for playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?
        );
    }


    Ok(
        policy_ids
    )
}


fn rewrite_positions_in_connection(
    connection: &Connection,
    playlist_id: i64,
    ordered_policy_ids: &[i64],
) -> Result<(), String> {

    if ordered_policy_ids.is_empty() {
        return Ok(());
    }


    let member_count =
        ordered_policy_ids.len() as i64;


    let maximum_position =
        connection
            .query_row(
                "SELECT COALESCE(MAX(position), 0)
                 FROM playlist_members
                 WHERE playlist_id = ?1",
                [playlist_id],
                |row| {
                    row.get::<_, i64>(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to determine maximum position while reordering playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    // Stage every current position above the entire existing position range
    // before assigning dense 1..N positions. Using only member_count as the
    // offset is insufficient when a playlist already contains gaps: for
    // example, positions 1 and 3 with two members would try to move 1 to 3 and
    // transiently violate UNIQUE (playlist_id, position).
    let staging_offset =
        maximum_position
            .checked_add(
                member_count
            )
            .and_then(
                |value| {
                    value.checked_add(
                        1
                    )
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "Playlist ID {} positions are too large to stage safely",
                        playlist_id,
                    )
                }
            )?;


    connection
        .execute(
            "UPDATE playlist_members
             SET position = position + ?1
             WHERE playlist_id = ?2",
            params![
                staging_offset,
                playlist_id,
            ],
        )
        .map_err(
            |error| {
                format!(
                    "Unable to stage positions while reordering playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )?;


    for (
        index,
        policy_id,
    ) in ordered_policy_ids
        .iter()
        .enumerate()
    {
        let position =
            index as i64 + 1;


        let changed =
            connection
                .execute(
                    "UPDATE playlist_members
                     SET position = ?1
                     WHERE playlist_id = ?2
                       AND policy_id = ?3",
                    params![
                        position,
                        playlist_id,
                        policy_id,
                    ],
                )
                .map_err(
                    |error| {
                        format!(
                            "Unable to assign position {} to policy ID {} in playlist ID {}: {}",
                            position,
                            policy_id,
                            playlist_id,
                            error,
                        )
                    }
                )?;


        if changed != 1 {
            return Err(
                format!(
                    "Expected to assign one playlist position for policy ID {} in playlist ID {}, updated {} rows",
                    policy_id,
                    playlist_id,
                    changed,
                )
            );
        }
    }


    Ok(())
}


fn validate_replacement_order(
    playlist_id: i64,
    existing_policy_ids: &[i64],
    ordered_policy_ids: &[i64],
) -> Result<(), String> {

    if existing_policy_ids.len()
        != ordered_policy_ids.len()
    {
        return Err(
            format!(
                "Replacement order for playlist ID {} contains {} policies; expected {}",
                playlist_id,
                ordered_policy_ids.len(),
                existing_policy_ids.len(),
            )
        );
    }


    let existing =
        existing_policy_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();

    let replacement =
        ordered_policy_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();


    if replacement.len()
        != ordered_policy_ids.len()
    {
        return Err(
            format!(
                "Replacement order for playlist ID {} contains duplicate policy IDs",
                playlist_id,
            )
        );
    }


    if existing != replacement {
        return Err(
            format!(
                "Replacement order for playlist ID {} does not contain exactly the current playlist members",
                playlist_id,
            )
        );
    }


    Ok(())
}


fn ensure_playlist_exists(
    connection: &Connection,
    playlist_id: i64,
) -> Result<(), String> {

    let count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM playlists
                 WHERE playlist_id = ?1",
                [playlist_id],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to check playlist ID {}: {}",
                        playlist_id,
                        error,
                    )
                }
            )?;


    if count == 1 {
        Ok(())

    } else {
        Err(
            format!(
                "Playlist ID {} does not exist",
                playlist_id,
            )
        )
    }
}


fn ensure_policy_exists(
    connection: &Connection,
    policy_id: i64,
) -> Result<(), String> {

    let count: i64 =
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM shader_policies
                 WHERE policy_id = ?1",
                [policy_id],
                |row| {
                    row.get(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to check policy ID {} before playlist operation: {}",
                        policy_id,
                        error,
                    )
                }
            )?;


    if count == 1 {
        Ok(())

    } else {
        Err(
            format!(
                "Policy ID {} does not exist",
                policy_id,
            )
        )
    }
}


fn ensure_playlist_name_available(
    connection: &Connection,
    playlist_name_key: &str,
    excluding_playlist_id: Option<i64>,
) -> Result<(), String> {

    let count: i64 =
        match excluding_playlist_id {

            Some(playlist_id) => {
                connection
                    .query_row(
                        "SELECT COUNT(*)
                         FROM playlists
                         WHERE playlist_name_key = ?1
                           AND playlist_id <> ?2",
                        params![
                            playlist_name_key,
                            playlist_id,
                        ],
                        |row| {
                            row.get(0)
                        },
                    )
            }

            None => {
                connection
                    .query_row(
                        "SELECT COUNT(*)
                         FROM playlists
                         WHERE playlist_name_key = ?1",
                        [playlist_name_key],
                        |row| {
                            row.get(0)
                        },
                    )
            }
        }
        .map_err(
            |error| {
                format!(
                    "Unable to check Playlist Name availability: {}",
                    error,
                )
            }
        )?;


    if count == 0 {
        Ok(())

    } else {
        Err(
            "A playlist with that Playlist Name already exists"
                .to_string()
        )
    }
}


fn playlist_name_for_id(
    connection: &Connection,
    playlist_id: i64,
) -> Result<String, String> {

    connection
        .query_row(
            "SELECT playlist_name
             FROM playlists
             WHERE playlist_id = ?1",
            [playlist_id],
            |row| {
                row.get(0)
            },
        )
        .optional()
        .map_err(
            |error| {
                format!(
                    "Unable to read Playlist Name for playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            }
        )?
        .ok_or_else(
            || {
                format!(
                    "Playlist ID {} does not exist",
                    playlist_id,
                )
            }
        )
}


fn normalized_playlist_name(
    playlist_name: &str,
) -> Result<String, String> {

    let playlist_name =
        playlist_name.trim();


    let length =
        playlist_name
            .chars()
            .count();


    if !(1..=128)
        .contains(
            &length
        )
    {
        return Err(
            format!(
                "Playlist Name must contain between 1 and 128 characters; found {}",
                length,
            )
        );
    }


    Ok(
        playlist_name.to_string()
    )
}


fn playlist_name_key(
    playlist_name: &str,
) -> Result<String, String> {

    let playlist_name =
        normalized_playlist_name(
            playlist_name
        )?;


    // Keep Playlist Name comparison behavior identical to the current
    // shader-policy implementation: Rust Unicode lowercase mapping supplies
    // the stable Schema V1 case-insensitive key.
    let key =
        playlist_name
            .chars()
            .flat_map(
                |character| {
                    character.to_lowercase()
                }
            )
            .collect::<String>();


    if key.is_empty() {
        return Err(
            "Playlist Name produced an empty comparison key"
                .to_string()
        );
    }


    Ok(
        key
    )
}


fn normalized_runtime_target(
    target: &str,
) -> Result<&str, String> {

    match target {
        "screensaver" | "wallpaper" => Ok(target),
        _ => Err(
            format!(
                "Playlist runtime target must be 'screensaver' or 'wallpaper'; found '{}'",
                target,
            )
        ),
    }
}


fn normalized_description(
    description: Option<&str>,
) -> Option<String> {

    description
        .map(
            str::trim
        )
        .filter(
            |value| {
                !value.is_empty()
            }
        )
        .map(
            str::to_string
        )
}


fn require_positive_id(
    kind: &str,
    id: i64,
) -> Result<(), String> {

    if id > 0 {
        Ok(())

    } else {
        Err(
            format!(
                "{} ID must be greater than zero; found {}",
                kind,
                id,
            )
        )
    }
}


fn expect_one_changed(
    changed: usize,
    operation: &str,
) -> Result<(), String> {

    match changed {

        1 => {
            Ok(())
        }

        0 => {
            Err(
                format!(
                    "Unable to {} because the requested row no longer exists",
                    operation,
                )
            )
        }

        count => {
            Err(
                format!(
                    "Unable to {} because {} rows were unexpectedly affected",
                    operation,
                    count,
                )
            )
        }
    }
}
