use std::time::{
    SystemTime,
    UNIX_EPOCH,
};


pub fn run(
) -> Result<(), String> {

    println!(
        "[PLAYLIST TEST] Starting non-destructive Playlist database-management tests."
    );


    let policy_ids =
        existing_policy_ids()?;


    if policy_ids.len() < 3 {
        return Err(
            format!(
                "At least 3 existing shader policies are required; found {}",
                policy_ids.len(),
            )
        );
    }


    let suffix =
        unique_suffix();


    let playlist_a_name =
        format!(
            "__Screenshaver Playlist Test A {}",
            suffix,
        );

    let playlist_b_name =
        format!(
            "__Screenshaver Playlist Test B {}",
            suffix,
        );


    let mut created_playlist_ids =
        Vec::new();


    let test_result =
        run_tests(
            &playlist_a_name,
            &playlist_b_name,
            &policy_ids,
            &mut created_playlist_ids,
        );


    let cleanup_result =
        cleanup_playlists(
            &created_playlist_ids
        );


    match (
        test_result,
        cleanup_result,
    ) {

        (Ok(()), Ok(())) => {
            Ok(())
        }

        (Err(test_error), Ok(())) => {
            Err(
                test_error
            )
        }

        (Ok(()), Err(cleanup_error)) => {
            Err(
                format!(
                    "Tests passed, but temporary-playlist cleanup failed: {}",
                    cleanup_error,
                )
            )
        }

        (
            Err(test_error),
            Err(cleanup_error),
        ) => {
            Err(
                format!(
                    "{}; temporary-playlist cleanup also failed: {}",
                    test_error,
                    cleanup_error,
                )
            )
        }
    }
}


fn run_tests(
    playlist_a_name: &str,
    playlist_b_name: &str,
    policy_ids: &[i64],
    created_playlist_ids: &mut Vec<i64>,
) -> Result<(), String> {

    let policy_a =
        policy_ids[0];

    let policy_b =
        policy_ids[1];

    let policy_c =
        policy_ids[2];


    println!(
        "[PLAYLIST TEST] Creating temporary playlists."
    );


    let playlist_a =
        crate::manage_playlists::create_playlist(
            playlist_a_name,
            Some(
                "Temporary developer Playlist API test"
            ),
        )?;

    created_playlist_ids.push(
        playlist_a
    );


    let playlist_b =
        crate::manage_playlists::create_playlist(
            playlist_b_name,
            None,
        )?;

    created_playlist_ids.push(
        playlist_b
    );


    assert_playlist_present(
        playlist_a,
        playlist_a_name,
    )?;

    assert_playlist_present(
        playlist_b,
        playlist_b_name,
    )?;


    println!(
        "[PLAYLIST TEST] Verifying rename and description updates."
    );


    let renamed_playlist_a =
        format!(
            "{} Renamed",
            playlist_a_name,
        );


    crate::manage_playlists::rename_playlist(
        playlist_a,
        &renamed_playlist_a,
    )?;


    crate::manage_playlists::update_playlist_description(
        playlist_b,
        Some(
            "Second temporary developer Playlist API test"
        ),
    )?;


    assert_playlist_present(
        playlist_a,
        &renamed_playlist_a,
    )?;


    println!(
        "[PLAYLIST TEST] Adding policies and verifying append order."
    );


    let add_result =
        crate::manage_playlists::add_policies(
            playlist_a,
            &[
                policy_a,
                policy_b,
                policy_c,
            ],
        )?;


    if add_result.added != 3
        || add_result.skipped_existing != 0
    {
        return Err(
            format!(
                "Initial bulk add returned added={}, skipped_existing={}; expected 3 and 0",
                add_result.added,
                add_result.skipped_existing,
            )
        );
    }


    assert_member_order(
        playlist_a,
        &[
            policy_a,
            policy_b,
            policy_c,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Verifying duplicate membership is ignored."
    );


    let duplicate_added =
        crate::manage_playlists::add_policy(
            playlist_a,
            policy_b,
        )?;


    if duplicate_added {
        return Err(
            "Adding an existing Playlist membership unexpectedly reported a new membership"
                .to_string()
        );
    }


    assert_member_order(
        playlist_a,
        &[
            policy_a,
            policy_b,
            policy_c,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Verifying one policy can belong to multiple playlists."
    );


    if !crate::manage_playlists::add_policy(
        playlist_b,
        policy_b,
    )? {
        return Err(
            "Adding policy to second temporary playlist unexpectedly reported an existing membership"
                .to_string()
        );
    }


    let reverse_lookup =
        crate::manage_playlists::playlists_for_policy(
            policy_b
        )?;


    let reverse_ids =
        reverse_lookup
            .iter()
            .map(
                |playlist| {
                    playlist.playlist_id
                }
            )
            .collect::<Vec<_>>();


    if !reverse_ids.contains(
        &playlist_a
    )
        || !reverse_ids.contains(
            &playlist_b
        )
    {
        return Err(
            format!(
                "Reverse Playlist lookup for policy ID {} did not return both temporary playlists",
                policy_b,
            )
        );
    }


    println!(
        "[PLAYLIST TEST] Verifying arbitrary persistent reorder."
    );


    crate::manage_playlists::replace_member_order(
        playlist_a,
        &[
            policy_c,
            policy_a,
            policy_b,
        ],
    )?;


    assert_member_order(
        playlist_a,
        &[
            policy_c,
            policy_a,
            policy_b,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Verifying move-to-position."
    );


    crate::manage_playlists::move_policy(
        playlist_a,
        policy_b,
        1,
    )?;


    assert_member_order(
        playlist_a,
        &[
            policy_b,
            policy_c,
            policy_a,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Removing middle member and verifying dense positions."
    );


    let removed =
        crate::manage_playlists::remove_policy(
            playlist_a,
            policy_c,
        )?;


    if !removed {
        return Err(
            format!(
                "Expected policy ID {} to be removed from playlist ID {}",
                policy_c,
                playlist_a,
            )
        );
    }


    assert_member_order(
        playlist_a,
        &[
            policy_b,
            policy_a,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Manufacturing a position gap and verifying explicit compaction."
    );


    manufacture_position_gap(
        playlist_a,
        policy_a,
        3,
    )?;


    assert_member_positions(
        playlist_a,
        &[
            (policy_b, 1),
            (policy_a, 3),
        ],
    )?;


    crate::manage_playlists::compact_playlist_positions(
        playlist_a
    )?;


    assert_member_order(
        playlist_a,
        &[
            policy_b,
            policy_a,
        ],
    )?;


    println!(
        "[PLAYLIST TEST] Verifying missing membership removal is harmless."
    );


    let removed_again =
        crate::manage_playlists::remove_policy(
            playlist_a,
            policy_c,
        )?;


    if removed_again {
        return Err(
            "Removing a missing Playlist membership unexpectedly reported a deletion"
                .to_string()
        );
    }


    println!(
        "[PLAYLIST TEST] Verifying runtime target filtering preserves canonical Playlist order."
    );

    assert_runtime_target_resolution(
        playlist_a,
        "screensaver",
    )?;

    assert_runtime_target_resolution(
        playlist_a,
        "wallpaper",
    )?;

    if crate::manage_playlists::playlist_render_policies(
        playlist_a,
        "unassigned",
    )
    .is_ok()
    {
        return Err(
            "Playlist runtime resolver unexpectedly accepted target 'unassigned'"
                .to_string()
        );
    }

    println!(
        "[PLAYLIST TEST] Core Playlist API tests passed; cleaning up temporary playlists."
    );


    Ok(())
}


fn existing_policy_ids(
) -> Result<Vec<i64>, String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while selecting policies for Playlist tests: {}",
                        error,
                    )
                }
            )?;


    let mut statement =
        connection
            .prepare(
                "SELECT policy_id
                 FROM shader_policies
                 ORDER BY policy_id
                 LIMIT 3"
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to prepare policy selection for Playlist tests: {}",
                        error,
                    )
                }
            )?;


    let rows =
        statement
            .query_map(
                [],
                |row| {
                    row.get::<_, i64>(0)
                },
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to select policies for Playlist tests: {}",
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
                        "Unable to decode policy ID for Playlist tests: {}",
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


fn assert_playlist_present(
    playlist_id: i64,
    expected_name: &str,
) -> Result<(), String> {

    let playlists =
        crate::manage_playlists::list_playlists()?;


    let Some(playlist) =
        playlists
            .iter()
            .find(
                |playlist| {
                    playlist.playlist_id
                        == playlist_id
                }
            )
    else {
        return Err(
            format!(
                "Playlist ID {} was not returned by list_playlists()",
                playlist_id,
            )
        );
    };


    if playlist.playlist_name
        != expected_name
    {
        return Err(
            format!(
                "Playlist ID {} has name '{}'; expected '{}'",
                playlist_id,
                playlist.playlist_name,
                expected_name,
            )
        );
    }


    Ok(())
}


fn assert_member_order(
    playlist_id: i64,
    expected_policy_ids: &[i64],
) -> Result<(), String> {

    let members =
        crate::manage_playlists::playlist_members(
            playlist_id
        )?;


    let actual_policy_ids =
        members
            .iter()
            .map(
                |member| {
                    member.policy_id
                }
            )
            .collect::<Vec<_>>();


    if actual_policy_ids
        != expected_policy_ids
    {
        return Err(
            format!(
                "Playlist ID {} member order is {:?}; expected {:?}",
                playlist_id,
                actual_policy_ids,
                expected_policy_ids,
            )
        );
    }


    for (
        index,
        member,
    ) in members
        .iter()
        .enumerate()
    {
        let expected_position =
            index as i64 + 1;


        if member.position
            != expected_position
        {
            return Err(
                format!(
                    "Playlist ID {} policy ID {} has position {}; expected {}",
                    playlist_id,
                    member.policy_id,
                    member.position,
                    expected_position,
                )
            );
        }
    }


    Ok(())
}


fn manufacture_position_gap(
    playlist_id: i64,
    policy_id: i64,
    new_position: i64,
) -> Result<(), String> {

    let connection =
        crate::open_database::open()
            .map_err(
                |error| {
                    format!(
                        "Unable to open database while manufacturing Playlist test gap: {}",
                        error,
                    )
                }
            )?;


    let changed =
        connection
            .execute(
                "UPDATE playlist_members
                 SET position = ?1
                 WHERE playlist_id = ?2
                   AND policy_id = ?3",
                rusqlite::params![
                    new_position,
                    playlist_id,
                    policy_id,
                ],
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to manufacture position gap for playlist ID {} policy ID {}: {}",
                        playlist_id,
                        policy_id,
                        error,
                    )
                }
            )?;


    if changed != 1 {
        return Err(
            format!(
                "Expected to update one membership while manufacturing a position gap; updated {} rows",
                changed,
            )
        );
    }


    Ok(())
}


fn assert_member_positions(
    playlist_id: i64,
    expected: &[(i64, i64)],
) -> Result<(), String> {

    let members =
        crate::manage_playlists::playlist_members(
            playlist_id
        )?;


    let actual =
        members
            .iter()
            .map(
                |member| {
                    (
                        member.policy_id,
                        member.position,
                    )
                }
            )
            .collect::<Vec<_>>();


    if actual
        != expected
    {
        return Err(
            format!(
                "Playlist ID {} member positions are {:?}; expected {:?}",
                playlist_id,
                actual,
                expected,
            )
        );
    }


    Ok(())
}


fn assert_runtime_target_resolution(
    playlist_id: i64,
    target: &str,
) -> Result<(), String> {

    let all_members =
        crate::manage_playlists::playlist_members(
            playlist_id
        )?;

    let expected =
        all_members
            .iter()
            .filter(
                |member| {
                    member.policy_target == target
                }
            )
            .map(
                |member| {
                    member.policy_id
                }
            )
            .collect::<Vec<_>>();

    let resolved =
        crate::manage_playlists::playlist_render_policies(
            playlist_id,
            target,
        )?;

    let actual =
        resolved
            .iter()
            .map(
                |member| {
                    member.policy_id
                }
            )
            .collect::<Vec<_>>();

    if actual != expected {
        return Err(
            format!(
                "Playlist ID {} runtime order for target '{}' is {:?}; expected {:?}",
                playlist_id,
                target,
                actual,
                expected,
            )
        );
    }

    if resolved
        .iter()
        .any(
            |member| {
                member.policy_target != target
            }
        )
    {
        return Err(
            format!(
                "Playlist ID {} runtime resolver returned a non-'{}' policy",
                playlist_id,
                target,
            )
        );
    }

    Ok(())
}


fn cleanup_playlists(
    playlist_ids: &[i64],
) -> Result<(), String> {

    let mut errors =
        Vec::new();


    for playlist_id in
        playlist_ids
            .iter()
            .rev()
    {
        if let Err(error) =
            crate::manage_playlists::delete_playlist(
                *playlist_id
            )
        {
            errors.push(
                format!(
                    "playlist ID {}: {}",
                    playlist_id,
                    error,
                )
            );
        }
    }


    if errors.is_empty() {
        println!(
            "[PLAYLIST TEST] Temporary playlists removed."
        );

        Ok(())

    } else {
        Err(
            errors.join(
                "; "
            )
        )
    }
}


fn unique_suffix(
) -> String {

    let nanos =
        SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .map(
                |duration| {
                    duration.as_nanos()
                }
            )
            .unwrap_or(0);


    format!(
        "{}-{}",
        std::process::id(),
        nanos,
    )
}
