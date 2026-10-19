//! Semantic conversion of extracted v001-v003 databases into the v004 target model.
//!
//! This module changes only in-memory MigrationData. The coordinator must
//! separately arrange a compatible TOML transition before database cutover.

use super::migration_data::{
    MigrationData, MigrationDefaultPalette, MigrationDefaultTexture,
    MigrationDisplayMode, MigrationIdleTimeout, MigrationPolicyTarget,
    MigrationTargetDefault,
};

/// Factory defaults for the new, independent Wallpaper target. These values
/// match the v003 initializer's Wallpaper defaults, not the user's old settings.
fn factory_wallpaper_defaults() -> MigrationTargetDefault {
    MigrationTargetDefault {
        idle_timeout: MigrationIdleTimeout::NotApplicable,
        animation_speed: 0.03,
        texture: MigrationDefaultTexture::Random,
        texture_primitives: 64,
        palette: MigrationDefaultPalette::Random,
    }
}

/// Convert a faithful v001, v002, or v003 extraction into three independent targets.
///
/// The historical reader initializes the Windowshader placeholders from the
/// Wallpaper values. For a full-screen source, no Windowshader policies exist;
/// its runtime selection is intentionally empty. For a windowed source, the
/// previous Wallpaper policies and selection become Windowshader's, while the
/// newly independent Wallpaper receives fresh factory defaults and no selection.
/// Playlist membership is never rewritten: policy IDs and member order survive.
///
/// This conversion does NOT authorize promotion: TOML enable-flag conversion
/// and rollback coordination remain mandatory before the staged DB cutover.
pub fn convert(data: &mut MigrationData) -> Result<(), String> {
    if !(1..=3).contains(&data.source.schema_version) {
        return Err(format!(
            "Legacy semantic conversion requires source schema 1, 2, or 3; received {}",
            data.source.schema_version
        ));
    }

    // Historical readers preserve the presentation mode in this common field.
    // Refuse unknown modes before changing any policy or target configuration.
    match data.application_defaults.wallpaper_display_format.trim() {
        "full_screen" => {
            data.runtime_configuration.windowshader =
                MigrationDisplayMode::Single { policy: None };
            data.target_defaults.windowshader = factory_wallpaper_defaults();
        }
        "windowed" => {
            for policy in &mut data.policies {
                if policy.target == MigrationPolicyTarget::Wallpaper {
                    policy.target = MigrationPolicyTarget::Windowshader;
                }
            }

            data.runtime_configuration.windowshader =
                data.runtime_configuration.wallpaper.clone();
            data.target_defaults.windowshader = data.target_defaults.wallpaper.clone();
            data.target_defaults.windowshader.idle_timeout =
                MigrationIdleTimeout::NotApplicable;

            data.runtime_configuration.wallpaper =
                MigrationDisplayMode::Single { policy: None };
            data.target_defaults.wallpaper = factory_wallpaper_defaults();
        }
        other => {
            return Err(format!(
                "Unsupported historical wallpaper_display_format {:?}; refusing lossy migration",
                other
            ));
        }
    }

    Ok(())
}
