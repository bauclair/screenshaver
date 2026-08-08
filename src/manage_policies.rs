use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{
    value,
    DocumentMut,
    Item,
    Table,
};


//
// ------------------------------------------------------------
// Public policy structures
// ------------------------------------------------------------
//

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum PolicyTarget {

    Screensaver,

    Wallpaper,
}


impl PolicyTarget {

    pub fn parse(
        value: &str,
    ) -> Result<Self, String> {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "screensaver" => {
                Ok(
                    Self::Screensaver
                )
            }

            "wallpaper" => {
                Ok(
                    Self::Wallpaper
                )
            }

            other => {
                Err(
                    format!(
                        "Unknown policy target '{}'; supported targets: screensaver, wallpaper",
                        other,
                    )
                )
            }
        }
    }


    pub fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver"
            }

            Self::Wallpaper => {
                "wallpaper"
            }
        }
    }


    pub fn table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_policies"
            }

            Self::Wallpaper => {
                "wallpaper_policies"
            }
        }
    }


    pub fn path_table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_external_paths"
            }

            Self::Wallpaper => {
                "wallpaper_external_paths"
            }
        }
    }


    fn legacy_path_table_name(
        self,
    ) -> &'static str {

        match self {
            Self::Screensaver => {
                "screensaver_shader_paths"
            }

            Self::Wallpaper => {
                "wallpaper_shader_paths"
            }
        }
    }
}


#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
)]
pub struct PolicyDefinition {

    pub texture:
        Option<String>,

    pub palette:
        Option<String>,

    pub fps:
        Option<u32>,

    pub speed:
        Option<f32>,

    pub render_scale:
        Option<f32>,

    pub anti_aliasing:
        Option<String>,

    pub dithering:
        Option<String>,

    pub color_precision:
        Option<String>,
}


impl PolicyDefinition {

    pub fn is_empty(
        &self,
    ) -> bool {

        self.texture.is_none()
            && self.palette.is_none()
            && self.fps.is_none()
            && self.speed.is_none()
            && self.render_scale.is_none()
            && self.anti_aliasing.is_none()
            && self.dithering.is_none()
            && self.color_precision.is_none()
    }
}


//
// ------------------------------------------------------------
// Public configuration operations
// ------------------------------------------------------------
//

pub fn add_policy(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
) -> Result<(), String> {

    add_policy_with_source_path(
        config_path,
        target,
        shader,
        properties,
        None,
    )
}


pub fn add_policy_with_source_path(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
    source_path: Option<&Path>,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    validate_properties(
        target,
        &properties
    )?;


    let normalized_source_path =
        normalize_source_path(
            source_path
        )?;


    let mut document =
        load_document(
            config_path
        )?;


    {
        let table =
            policy_table_mut(
                &mut document,
                target,
            )?;


        if let Some(existing_key) =
            matching_shader_key(
                table,
                &shader,
            )
        {
            return Err(
                format!(
                    "Shader '{}' already has an policy in [{}]",
                    existing_key,
                    target.table_name(),
                )
            );
        }


        let specification =
            format_policy(
                &properties
            );


        table[
            &shader
        ] =
            value(
                specification
            );
    }


    if let Some(source_path) =
        normalized_source_path
    {
        set_source_path_metadata(
            &mut document,
            target,
            &shader,
            Some(
                &source_path
            ),
        )?;
    }


    save_document(
        config_path,
        &document,
    )
}

pub fn policy_exists(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
) -> Result<bool, String> {

    let shader =
        normalized_shader_name(
            shader
        )?;

    let document =
        load_document(
            config_path
        )?;

    let Some(table) =
        policy_table(
            &document,
            target,
        )?
    else {
        return Ok(false);
    };

    Ok(
        matching_shader_key(
            table,
            &shader,
        )
        .is_some()
    )
}



pub fn reconcile_shader_move(
    config_path: &Path,
    shader: &str,
    destination_path: &Path,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;

    if !destination_path.is_absolute() {
        return Err(
            format!(
                "Moved shader path must be absolute: {}",
                destination_path.display(),
            )
        );
    }

    let mut document =
        load_document(
            config_path
        )?;

    let screensaver_managed_path =
        crate::locate_paths::screensaver_shader_dir()
            .join(
                &shader
            );

    let wallpaper_managed_path =
        crate::locate_paths::wallpaper_shader_dir()
            .join(
                &shader
            );

    for target in [
        PolicyTarget::Screensaver,
        PolicyTarget::Wallpaper,
    ] {
        let policy_exists =
            policy_table(
                &document,
                target,
            )?
            .and_then(
                |table| {
                    matching_shader_key(
                        table,
                        &shader,
                    )
                }
            )
            .is_some();

        if !policy_exists {
            continue;
        }

        let managed_path =
            match target {
                PolicyTarget::Screensaver =>
                    &screensaver_managed_path,

                PolicyTarget::Wallpaper =>
                    &wallpaper_managed_path,
            };

        if destination_path == managed_path {
            set_source_path_metadata(
                &mut document,
                target,
                &shader,
                None,
            )?;
        } else {
            let destination_text =
                destination_path
                    .to_str()
                    .ok_or_else(
                        || {
                            format!(
                                "Moved shader path is not valid UTF-8: {}",
                                destination_path.display(),
                            )
                        }
                    )?;

            set_source_path_metadata(
                &mut document,
                target,
                &shader,
                Some(
                    destination_text
                ),
            )?;
        }
    }

    save_document(
        config_path,
        &document,
    )
}


pub fn policy_source_path(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
) -> Result<Option<PathBuf>, String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    let document =
        load_document(
            config_path
        )?;


    for table_name in [
        target.path_table_name(),
        target.legacy_path_table_name(),
    ] {
        let Some(item) =
            document.get(
                table_name
            )
        else {
            continue;
        };


        let table =
            item.as_table()
                .ok_or_else(
                    || {
                        format!(
                            "[{}] exists but is not a TOML table",
                            table_name,
                        )
                    }
                )?;


        let Some(existing_key) =
            matching_shader_key(
                table,
                &shader,
            )
        else {
            continue;
        };


        let raw_path =
            table
                .get(
                    &existing_key
                )
                .and_then(
                    |item| {
                        item.as_value()
                    }
                )
                .and_then(
                    |value| {
                        value.as_str()
                    }
                )
                .ok_or_else(
                    || {
                        format!(
                            "Shader path '{}' in [{}] must be a TOML string",
                            existing_key,
                            table_name,
                        )
                    }
                )?;


        let path =
            PathBuf::from(
                raw_path
            );


        if !path.is_absolute() {
            return Err(
                format!(
                    "Shader path '{}' in [{}] must be absolute: {}",
                    existing_key,
                    table_name,
                    raw_path,
                )
            );
        }


        return Ok(
            Some(
                path
            )
        );
    }


    Ok(
        None
    )
}

pub fn replace_policy(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
) -> Result<(), String> {

    // Legacy callers update only policy properties.  Preserve any
    // existing external source-path metadata.
    replace_policy_internal(
        config_path,
        target,
        shader,
        properties,
        SourcePathUpdate::Preserve,
    )
}


pub fn replace_policy_with_source_path(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
    source_path: Option<&Path>,
) -> Result<(), String> {

    let normalized_source_path =
        normalize_source_path(
            source_path
        )?;


    replace_policy_internal(
        config_path,
        target,
        shader,
        properties,
        SourcePathUpdate::Set(
            normalized_source_path
        ),
    )
}


enum SourcePathUpdate {
    Preserve,
    Set(
        Option<String>
    ),
}


fn replace_policy_internal(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
    properties: PolicyDefinition,
    source_path_update: SourcePathUpdate,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    validate_properties(
        target,
        &properties
    )?;


    let mut document =
        load_document(
            config_path
        )?;


    {
        let table =
            policy_table_mut(
                &mut document,
                target,
            )?;


        let existing_key =
            matching_shader_key(
                table,
                &shader,
            )
            .ok_or_else(
                || {
                    format!(
                        "Shader '{}' does not have an policy in [{}]",
                        shader,
                        target.table_name(),
                    )
                }
            )?;


        table.remove(
            &existing_key
        );


        table[
            &shader
        ] =
            value(
                format_policy(
                    &properties
                )
            );
    }


    match source_path_update {
        SourcePathUpdate::Preserve => {}

        SourcePathUpdate::Set(
            source_path
        ) => {
            set_source_path_metadata(
                &mut document,
                target,
                &shader,
                source_path.as_deref(),
            )?;
        }
    }


    save_document(
        config_path,
        &document,
    )
}

pub fn delete_policy(
    config_path: &Path,
    target: PolicyTarget,
    shader: &str,
) -> Result<(), String> {

    let shader =
        normalized_shader_name(
            shader
        )?;


    let mut document =
        load_document(
            config_path
        )?;


    let table =
        policy_table_mut(
            &mut document,
            target,
        )?;


    let existing_key =
        matching_shader_key(
            table,
            &shader,
        )
        .ok_or_else(
            || {
                format!(
                    "Shader '{}' does not have an policy in [{}]",
                    shader,
                    target.table_name(),
                )
            }
        )?;


    table.remove(
        &existing_key
    );


    remove_source_path_metadata(
        &mut document,
        target,
        &shader,
    )?;


    save_document(
        config_path,
        &document,
    )
}


pub fn list_policies(
    config_path: &Path,
    target: Option<PolicyTarget>,
) -> Result<(), String> {

    let document =
        load_document(
            config_path
        )?;


    match target {
        Some(target) => {
            print_policy_table(
                &document,
                target,
            )?;
        }

        None => {
            print_policy_table(
                &document,
                PolicyTarget::Screensaver,
            )?;

            println!();

            print_policy_table(
                &document,
                PolicyTarget::Wallpaper,
            )?;
        }
    }


    Ok(())
}


//
// ------------------------------------------------------------
// TOML document handling
// ------------------------------------------------------------
//

fn load_document(
    path: &Path,
) -> Result<DocumentMut, String> {

    let text =
        fs::read_to_string(
            path
        )
        .map_err(
            |error| {
                format!(
                    "Unable to read configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )?;


    text.parse::<DocumentMut>()
        .map_err(
            |error| {
                format!(
                    "Unable to parse configuration file {} ({})",
                    path.display(),
                    error,
                )
            }
        )
}


fn save_document(
    path: &Path,
    document: &DocumentMut,
) -> Result<(), String> {

    fs::write(
        path,
        document.to_string(),
    )
    .map_err(
        |error| {
            format!(
                "Unable to write configuration file {} ({})",
                path.display(),
                error,
            )
        }
    )
}


fn policy_table_mut<'a>(
    document: &'a mut DocumentMut,
    target: PolicyTarget,
) -> Result<&'a mut Table, String> {

    let table_name =
        target.table_name();


    if !document.contains_key(
        table_name
    ) {
        let mut table =
            Table::new();

        table.set_implicit(
            false
        );

        document[
            table_name
        ] =
            Item::Table(
                table
            );
    }


    document[
        table_name
    ]
        .as_table_mut()
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn policy_table<'a>(
    document: &'a DocumentMut,
    target: PolicyTarget,
) -> Result<Option<&'a Table>, String> {

    let table_name =
        target.table_name();


    let Some(item) =
        document.get(
            table_name
        )
    else {
        return Ok(
            None
        );
    };


    item.as_table()
        .map(
            Some
        )
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn source_path_table_mut<'a>(
    document: &'a mut DocumentMut,
    target: PolicyTarget,
) -> Result<&'a mut Table, String> {

    let table_name =
        target.path_table_name();


    if !document.contains_key(
        table_name
    ) {
        let mut table =
            Table::new();

        table.set_implicit(
            false
        );

        document[
            table_name
        ] =
            Item::Table(
                table
            );
    }


    document[
        table_name
    ]
        .as_table_mut()
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn source_path_table<'a>(
    document: &'a DocumentMut,
    target: PolicyTarget,
) -> Result<Option<&'a Table>, String> {

    let table_name =
        target.path_table_name();


    let Some(item) =
        document.get(
            table_name
        )
    else {
        return Ok(
            None
        );
    };


    item.as_table()
        .map(
            Some
        )
        .ok_or_else(
            || {
                format!(
                    "[{}] exists but is not a TOML table",
                    table_name,
                )
            }
        )
}


fn normalize_source_path(
    source_path: Option<&Path>,
) -> Result<Option<String>, String> {

    let Some(source_path) =
        source_path
    else {
        return Ok(
            None
        );
    };


    if !source_path.is_absolute() {
        return Err(
            format!(
                "External shader path must be absolute: {}",
                source_path.display(),
            )
        );
    }


    let source_path =
        source_path
            .to_str()
            .ok_or_else(
                || {
                    format!(
                        "External shader path is not valid UTF-8: {}",
                        source_path.display(),
                    )
                }
            )?
            .trim();


    if source_path.is_empty() {
        return Err(
            "External shader path may not be empty"
                .to_string()
        );
    }


    Ok(
        Some(
            source_path.to_string()
        )
    )
}


fn set_source_path_metadata(
    document: &mut DocumentMut,
    target: PolicyTarget,
    shader: &str,
    source_path: Option<&str>,
) -> Result<(), String> {

    match source_path {
        Some(source_path) => {
            let table =
                source_path_table_mut(
                    document,
                    target,
                )?;


            if let Some(existing_key) =
                matching_shader_key(
                    table,
                    shader,
                )
            {
                table.remove(
                    &existing_key
                );
            }


            table[
                shader
            ] =
                value(
                    source_path
                );


            remove_source_path_metadata_from_named_table(
                document,
                target.legacy_path_table_name(),
                shader,
            )?;
        }

        None => {
            remove_source_path_metadata(
                document,
                target,
                shader,
            )?;
        }
    }


    Ok(())
}


fn remove_source_path_metadata(
    document: &mut DocumentMut,
    target: PolicyTarget,
    shader: &str,
) -> Result<(), String> {

    remove_source_path_metadata_from_named_table(
        document,
        target.path_table_name(),
        shader,
    )?;


    remove_source_path_metadata_from_named_table(
        document,
        target.legacy_path_table_name(),
        shader,
    )
}


fn remove_source_path_metadata_from_named_table(
    document: &mut DocumentMut,
    table_name: &str,
    shader: &str,
) -> Result<(), String> {

    let should_remove_table =
        {
            let Some(item) =
                document.get_mut(
                    table_name
                )
            else {
                return Ok(());
            };


            let table =
                item.as_table_mut()
                    .ok_or_else(
                        || {
                            format!(
                                "[{}] exists but is not a TOML table",
                                table_name,
                            )
                        }
                    )?;


            if let Some(existing_key) =
                matching_shader_key(
                    table,
                    shader,
                )
            {
                table.remove(
                    &existing_key
                );
            }


            table.is_empty()
        };


    if should_remove_table {
        document.remove(
            table_name
        );
    }


    Ok(())
}

//
// ------------------------------------------------------------
// Policy formatting and validation
// ------------------------------------------------------------
//

fn normalized_shader_name(
    shader: &str,
) -> Result<String, String> {

    let shader =
        shader.trim();


    if shader.is_empty() {
        return Err(
            "Shader name may not be empty"
                .to_string()
        );
    }


    Ok(
        shader.to_string()
    )
}


fn validate_properties(
    target: PolicyTarget,
    properties: &PolicyDefinition,
) -> Result<(), String> {

    if properties.is_empty() {
        return Err(
            "A policy must define at least one property"
                .to_string()
        );
    }


    if let Some(texture) =
        properties.texture.as_deref()
    {
        validate_property_text(
            "texture",
            texture,
        )?;
    }


    if let Some(palette) =
        properties.palette.as_deref()
    {
        validate_property_text(
            "palette",
            palette,
        )?;
    }


    if let Some(fps) =
        properties.fps
    {
        if !(crate::define_constants::MIN_RENDER_FPS
            ..=crate::define_constants::MAX_RENDER_FPS)
            .contains(
                &fps
            )
        {
            return Err(
                format!(
                    "FPS policy {} is outside the supported range {}-{}",
                    fps,
                    crate::define_constants::MIN_RENDER_FPS,
                    crate::define_constants::MAX_RENDER_FPS,
                )
            );
        }
    }


    if let Some(speed) =
        properties.speed
    {
        let (
            minimum,
            maximum,
        ) =
            match target {
                PolicyTarget::Screensaver => (
                    crate::define_constants::SCREENSAVER_SPEED_MIN,
                    crate::define_constants::SCREENSAVER_SPEED_MAX,
                ),

                PolicyTarget::Wallpaper => (
                    crate::define_constants::WALLPAPER_SPEED_MIN,
                    crate::define_constants::WALLPAPER_SPEED_MAX,
                ),
            };


        if !speed.is_finite()
            || !(minimum..=maximum)
                .contains(
                    &speed
                )
        {
            return Err(
                format!(
                    "Speed policy {} for {} is outside the supported range {}-{}",
                    speed,
                    target.name(),
                    minimum,
                    maximum,
                )
            );
        }
    }


    if let Some(render_scale) =
        properties.render_scale
    {
        if !render_scale.is_finite()
            || !(crate::define_constants::RENDER_SCALE_MIN
                ..=crate::define_constants::RENDER_SCALE_MAX)
                .contains(
                    &render_scale
                )
        {
            return Err(
                format!(
                    "Render-scale policy {} is outside the supported range {:.2}-{:.2}",
                    render_scale,
                    crate::define_constants::RENDER_SCALE_MIN,
                    crate::define_constants::RENDER_SCALE_MAX,
                )
            );
        }
    }


    if let Some(value) =
        properties.anti_aliasing.as_deref()
    {
        validate_named_policy_value(
            "anti_aliasing",
            value,
            &["off", "fxaa"],
        )?;
    }

    if let Some(value) =
        properties.dithering.as_deref()
    {
        validate_named_policy_value(
            "dithering",
            value,
            &["off", "subtle"],
        )?;
    }

    if let Some(value) =
        properties.color_precision.as_deref()
    {
        validate_named_policy_value(
            "color_precision",
            value,
            &["auto", "standard", "high"],
        )?;
    }

    Ok(())
}


fn validate_property_text(
    property_name: &str,
    value: &str,
) -> Result<(), String> {

    let value =
        value.trim();


    if value.is_empty() {
        return Err(
            format!(
                "Policy property '{}' requires a value",
                property_name,
            )
        );
    }


    if value.chars()
        .any(
            char::is_whitespace
        )
    {
        return Err(
            format!(
                "Policy property '{}' may not contain whitespace: '{}'",
                property_name,
                value,
            )
        );
    }


    Ok(())
}


fn validate_named_policy_value(
    property_name: &str,
    value: &str,
    supported_values: &[&str],
) -> Result<(), String> {
    validate_property_text(
        property_name,
        value,
    )?;

    let normalized =
        value.trim()
            .to_ascii_lowercase();

    if supported_values.contains(
        &normalized.as_str()
    ) {
        return Ok(());
    }

    Err(
        format!(
            "Unsupported {} policy value '{}'; supported values: {}",
            property_name,
            value,
            supported_values.join(", "),
        )
    )
}


fn format_policy(
    properties: &PolicyDefinition,
) -> String {

    let mut tokens =
        Vec::with_capacity(
            8
        );


    if let Some(texture) =
        properties.texture.as_deref()
    {
        tokens.push(
            format!(
                "texture:{}",
                texture.trim()
                    .to_ascii_lowercase(),
            )
        );
    }


    if let Some(palette) =
        properties.palette.as_deref()
    {
        tokens.push(
            format!(
                "palette:{}",
                palette.trim()
                    .to_ascii_lowercase(),
            )
        );
    }


    if let Some(fps) =
        properties.fps
    {
        tokens.push(
            format!(
                "fps:{}",
                fps,
            )
        );
    }


    if let Some(speed) =
        properties.speed
    {
        tokens.push(
            format!(
                "speed:{}",
                format_speed(
                    speed
                ),
            )
        );
    }


    if let Some(render_scale) =
        properties.render_scale
    {
        tokens.push(
            format!(
                "render_scale:{}",
                format_speed(
                    render_scale
                ),
            )
        );
    }


    if let Some(value) =
        properties.anti_aliasing.as_deref()
    {
        tokens.push(
            format!(
                "anti_aliasing:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(value) =
        properties.dithering.as_deref()
    {
        tokens.push(
            format!(
                "dithering:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    if let Some(value) =
        properties.color_precision.as_deref()
    {
        tokens.push(
            format!(
                "color_precision:{}",
                value.trim().to_ascii_lowercase(),
            )
        );
    }

    tokens.join(
        " "
    )
}


fn format_speed(
    speed: f32,
) -> String {

    let mut value =
        speed.to_string();


    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }


        if value.ends_with('.') {
            value.push(
                '0'
            );
        }
    }


    value
}


//
// ------------------------------------------------------------
// Policy lookup and display
// ------------------------------------------------------------
//

fn matching_shader_key(
    table: &Table,
    shader: &str,
) -> Option<String> {

    table
        .iter()
        .find_map(
            |(
                key,
                _,
            )| {
                if key.eq_ignore_ascii_case(
                    shader
                ) {
                    Some(
                        key.to_string()
                    )
                } else {
                    None
                }
            }
        )
}


#[derive(Debug)]
struct PolicyRow {

    shader:
        String,

    texture:
        String,

    palette:
        String,

    fps:
        String,

    speed:
        String,

    render_scale:
        String,

    anti_aliasing:
        String,

    dithering:
        String,

    color_precision:
        String,
}


#[derive(Debug)]
struct PolicyTableLayout {

    shader_width:
        usize,

    texture_width:
        usize,

    palette_width:
        usize,

    fps_width:
        usize,

    speed_width:
        usize,

    render_scale_width:
        usize,

    anti_aliasing_width:
        usize,

    dithering_width:
        usize,

    color_precision_width:
        usize,
}


fn print_policy_table(
    document: &DocumentMut,
    target: PolicyTarget,
) -> Result<(), String> {

    println!(
        "{} Policies",
        display_target_name(
            target
        )
    );

    println!();


    let Some(table) =
        policy_table(
            document,
            target,
        )?
    else {
        println!(
            "No {} policies defined.",
            target.name(),
        );

        return Ok(());
    };


    let rows =
        collect_policy_rows(
            table,
            target,
        )?;


    if rows.is_empty() {
        println!(
            "No {} policies defined.",
            target.name(),
        );

        return Ok(());
    }


    let layout =
        calculate_policy_table_layout(
            &rows
        );


    print_policy_table_header(
        &layout
    );


    for row in
        &rows
    {
        println!(
            "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}  {:>render_scale_width$}  {:<anti_aliasing_width$}  {:<dithering_width$}  {:<color_precision_width$}",
            row.shader,
            row.texture,
            row.palette,
            row.fps,
            row.speed,
            row.render_scale,
            row.anti_aliasing,
            row.dithering,
            row.color_precision,
            shader_width = layout.shader_width,
            texture_width = layout.texture_width,
            palette_width = layout.palette_width,
            fps_width = layout.fps_width,
            speed_width = layout.speed_width,
            render_scale_width = layout.render_scale_width,
            anti_aliasing_width = layout.anti_aliasing_width,
            dithering_width = layout.dithering_width,
            color_precision_width = layout.color_precision_width,
        );
    }


    Ok(())
}


fn collect_policy_rows(
    table: &Table,
    target: PolicyTarget,
) -> Result<Vec<PolicyRow>, String> {

    let mut entries =
        table.iter()
            .collect::<Vec<_>>();


    entries.sort_by(
        |(
            left,
            _,
        ),
         (
            right,
            _,
        )| {
            left.to_ascii_lowercase()
                .cmp(
                    &right.to_ascii_lowercase()
                )
        }
    );


    entries
        .into_iter()
        .map(
            |(
                shader,
                item,
            )| {
                let specification =
                    item.as_str()
                        .ok_or_else(
                            || {
                                format!(
                                    "Policy '{}' in [{}] is not a string",
                                    shader,
                                    target.table_name(),
                                )
                            }
                        )?;


                let speed =
                    match policy_property_value(
                        specification,
                        "speed",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid speed '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;


                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                let render_scale =
                    match policy_property_value(
                        specification,
                        "render_scale",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid render_scale '{}' for policy '{}' in [{}]",
                                                value,
                                                shader,
                                                target.table_name(),
                                            )
                                        }
                                    )?;

                            if !parsed.is_finite()
                                || !(crate::define_constants::RENDER_SCALE_MIN
                                    ..=crate::define_constants::RENDER_SCALE_MAX)
                                    .contains(&parsed)
                            {
                                return Err(
                                    format!(
                                        "render_scale '{}' for policy '{}' in [{}] is outside the supported range {:.2}-{:.2}",
                                        value,
                                        shader,
                                        target.table_name(),
                                        crate::define_constants::RENDER_SCALE_MIN,
                                        crate::define_constants::RENDER_SCALE_MAX,
                                    )
                                );
                            }

                            format!(
                                "{:.3}",
                                parsed,
                            )
                        }

                        None => {
                            "-".to_string()
                        }
                    };


                Ok(
                    PolicyRow {
                        shader:
                            shader.to_string(),

                        texture:
                            policy_property_value(
                                specification,
                                "texture",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        palette:
                            policy_property_value(
                                specification,
                                "palette",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        fps:
                            policy_property_value(
                                specification,
                                "fps",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        speed,

                        render_scale,

                        anti_aliasing:
                            policy_property_value(
                                specification,
                                "anti_aliasing",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        dithering:
                            policy_property_value(
                                specification,
                                "dithering",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        color_precision:
                            policy_property_value(
                                specification,
                                "color_precision",
                            )
                            .unwrap_or("-")
                            .to_string(),
                    }
                )
            }
        )
        .collect()
}


fn calculate_policy_table_layout(
    rows: &[PolicyRow],
) -> PolicyTableLayout {

    PolicyTableLayout {
        shader_width:
            rows.iter()
                .map(
                    |row| {
                        row.shader.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Shader".len()
                ),

        texture_width:
            rows.iter()
                .map(
                    |row| {
                        row.texture.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Texture".len()
                ),

        palette_width:
            rows.iter()
                .map(
                    |row| {
                        row.palette.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Palette".len()
                ),

        fps_width:
            rows.iter()
                .map(
                    |row| {
                        row.fps.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "FPS".len()
                ),

        speed_width:
            rows.iter()
                .map(
                    |row| {
                        row.speed.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Speed".len()
                ),

        render_scale_width:
            rows.iter()
                .map(
                    |row| {
                        row.render_scale.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Render Scale".len()
                ),

        anti_aliasing_width:
            rows.iter()
                .map(
                    |row| {
                        row.anti_aliasing.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Anti-aliasing".len()
                ),

        dithering_width:
            rows.iter()
                .map(
                    |row| {
                        row.dithering.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Dithering".len()
                ),

        color_precision_width:
            rows.iter()
                .map(
                    |row| {
                        row.color_precision.len()
                    }
                )
                .max()
                .unwrap_or(0)
                .max(
                    "Color precision".len()
                ),
    }
}


fn print_policy_table_header(
    layout: &PolicyTableLayout,
) {

    println!(
        "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}  {:>render_scale_width$}  {:<anti_aliasing_width$}  {:<dithering_width$}  {:<color_precision_width$}",
        "Shader",
        "Texture",
        "Palette",
        "FPS",
        "Speed",
        "Render Scale",
        "Anti-aliasing",
        "Dithering",
        "Color precision",
        shader_width = layout.shader_width,
        texture_width = layout.texture_width,
        palette_width = layout.palette_width,
        fps_width = layout.fps_width,
        speed_width = layout.speed_width,
        render_scale_width = layout.render_scale_width,
        anti_aliasing_width = layout.anti_aliasing_width,
        dithering_width = layout.dithering_width,
        color_precision_width = layout.color_precision_width,
    );


    println!(
        "{}  {}  {}  {}  {}  {}  {}  {}  {}",
        "-".repeat(
            layout.shader_width
        ),
        "-".repeat(
            layout.texture_width
        ),
        "-".repeat(
            layout.palette_width
        ),
        "-".repeat(
            layout.fps_width
        ),
        "-".repeat(
            layout.speed_width
        ),
        "-".repeat(
            layout.render_scale_width
        ),
        "-".repeat(
            layout.anti_aliasing_width
        ),
        "-".repeat(
            layout.dithering_width
        ),
        "-".repeat(
            layout.color_precision_width
        ),
    );
}


fn policy_property_value<'a>(
    specification: &'a str,
    property: &str,
) -> Option<&'a str> {

    specification
        .split_whitespace()
        .find_map(
            |token| {
                let (
                    name,
                    value,
                ) =
                    token.split_once(
                        ':'
                    )?;


                if name.eq_ignore_ascii_case(
                    property
                ) {
                    Some(
                        value
                    )
                } else {
                    None
                }
            }
        )
}


fn display_target_name(
    target: PolicyTarget,
) -> &'static str {

    match target {
        PolicyTarget::Screensaver => {
            "Screensaver"
        }

        PolicyTarget::Wallpaper => {
            "Wallpaper"
        }
    }
}

