use std::fs;
use std::path::Path;

use toml_edit::{
    value,
    DocumentMut,
    Item,
    Table,
};


//
// ------------------------------------------------------------
// Public override structures
// ------------------------------------------------------------
//

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum OverrideTarget {

    Screensaver,

    Wallpaper,
}


impl OverrideTarget {

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
                        "Unknown override target '{}'; supported targets: screensaver, wallpaper",
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
                "screensaver_overrides"
            }

            Self::Wallpaper => {
                "wallpaper_overrides"
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
pub struct OverrideProperties {

    pub texture:
        Option<String>,

    pub palette:
        Option<String>,

    pub fps:
        Option<u32>,

    pub speed:
        Option<f32>,
}


impl OverrideProperties {

    pub fn is_empty(
        &self,
    ) -> bool {

        self.texture.is_none()
            && self.palette.is_none()
            && self.fps.is_none()
            && self.speed.is_none()
    }
}


//
// ------------------------------------------------------------
// Public configuration operations
// ------------------------------------------------------------
//

pub fn add_override(
    config_path: &Path,
    target: OverrideTarget,
    shader: &str,
    properties: OverrideProperties,
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


    let table =
        override_table_mut(
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
                "Shader '{}' already has an override in [{}]",
                existing_key,
                target.table_name(),
            )
        );
    }


    let specification =
        format_override(
            &properties
        );


    table[
        &shader
    ] =
        value(
            specification
        );


    save_document(
        config_path,
        &document,
    )
}


pub fn override_exists(
    config_path: &Path,
    target: OverrideTarget,
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
        override_table(
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


pub fn replace_override(
    config_path: &Path,
    target: OverrideTarget,
    shader: &str,
    properties: OverrideProperties,
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

    let table =
        override_table_mut(
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
                    "Shader '{}' does not have an override in [{}]",
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
            format_override(
                &properties
            )
        );

    save_document(
        config_path,
        &document,
    )
}


pub fn delete_override(
    config_path: &Path,
    target: OverrideTarget,
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
        override_table_mut(
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
                    "Shader '{}' does not have an override in [{}]",
                    shader,
                    target.table_name(),
                )
            }
        )?;


    table.remove(
        &existing_key
    );


    save_document(
        config_path,
        &document,
    )
}


pub fn list_overrides(
    config_path: &Path,
    target: Option<OverrideTarget>,
) -> Result<(), String> {

    let document =
        load_document(
            config_path
        )?;


    match target {
        Some(target) => {
            print_override_table(
                &document,
                target,
            )?;
        }

        None => {
            print_override_table(
                &document,
                OverrideTarget::Screensaver,
            )?;

            println!();

            print_override_table(
                &document,
                OverrideTarget::Wallpaper,
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


fn override_table_mut<'a>(
    document: &'a mut DocumentMut,
    target: OverrideTarget,
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


fn override_table<'a>(
    document: &'a DocumentMut,
    target: OverrideTarget,
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


//
// ------------------------------------------------------------
// Override formatting and validation
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
    target: OverrideTarget,
    properties: &OverrideProperties,
) -> Result<(), String> {

    if properties.is_empty() {
        return Err(
            "An override must define at least one property"
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
                    "FPS override {} is outside the supported range {}-{}",
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
                OverrideTarget::Screensaver => (
                    crate::define_constants::SCREENSAVER_SPEED_MIN,
                    crate::define_constants::SCREENSAVER_SPEED_MAX,
                ),

                OverrideTarget::Wallpaper => (
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
                    "Speed override {} for {} is outside the supported range {}-{}",
                    speed,
                    target.name(),
                    minimum,
                    maximum,
                )
            );
        }
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
                "Override property '{}' requires a value",
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
                "Override property '{}' may not contain whitespace: '{}'",
                property_name,
                value,
            )
        );
    }


    Ok(())
}


fn format_override(
    properties: &OverrideProperties,
) -> String {

    let mut tokens =
        Vec::with_capacity(
            4
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
// Override lookup and display
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
struct OverrideRow {

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
}


#[derive(Debug)]
struct OverrideTableLayout {

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
}


fn print_override_table(
    document: &DocumentMut,
    target: OverrideTarget,
) -> Result<(), String> {

    println!(
        "{} Overrides",
        display_target_name(
            target
        )
    );

    println!();


    let Some(table) =
        override_table(
            document,
            target,
        )?
    else {
        println!(
            "No {} overrides defined.",
            target.name(),
        );

        return Ok(());
    };


    let rows =
        collect_override_rows(
            table,
            target,
        )?;


    if rows.is_empty() {
        println!(
            "No {} overrides defined.",
            target.name(),
        );

        return Ok(());
    }


    let layout =
        calculate_override_table_layout(
            &rows
        );


    print_override_table_header(
        &layout
    );


    for row in
        &rows
    {
        println!(
            "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}",
            row.shader,
            row.texture,
            row.palette,
            row.fps,
            row.speed,
            shader_width = layout.shader_width,
            texture_width = layout.texture_width,
            palette_width = layout.palette_width,
            fps_width = layout.fps_width,
            speed_width = layout.speed_width,
        );
    }


    Ok(())
}


fn collect_override_rows(
    table: &Table,
    target: OverrideTarget,
) -> Result<Vec<OverrideRow>, String> {

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
                                    "Override '{}' in [{}] is not a string",
                                    shader,
                                    target.table_name(),
                                )
                            }
                        )?;


                let speed =
                    match override_property_value(
                        specification,
                        "speed",
                    ) {
                        Some(value) => {
                            let parsed =
                                value.parse::<f32>()
                                    .map_err(
                                        |_| {
                                            format!(
                                                "Invalid speed '{}' for override '{}' in [{}]",
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


                Ok(
                    OverrideRow {
                        shader:
                            shader.to_string(),

                        texture:
                            override_property_value(
                                specification,
                                "texture",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        palette:
                            override_property_value(
                                specification,
                                "palette",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        fps:
                            override_property_value(
                                specification,
                                "fps",
                            )
                            .unwrap_or("-")
                            .to_string(),

                        speed,
                    }
                )
            }
        )
        .collect()
}


fn calculate_override_table_layout(
    rows: &[OverrideRow],
) -> OverrideTableLayout {

    OverrideTableLayout {
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
    }
}


fn print_override_table_header(
    layout: &OverrideTableLayout,
) {

    println!(
        "{:<shader_width$}  {:<texture_width$}  {:<palette_width$}  {:>fps_width$}  {:>speed_width$}",
        "Shader",
        "Texture",
        "Palette",
        "FPS",
        "Speed",
        shader_width = layout.shader_width,
        texture_width = layout.texture_width,
        palette_width = layout.palette_width,
        fps_width = layout.fps_width,
        speed_width = layout.speed_width,
    );


    println!(
        "{}  {}  {}  {}  {}",
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
    );
}


fn override_property_value<'a>(
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
    target: OverrideTarget,
) -> &'static str {

    match target {
        OverrideTarget::Screensaver => {
            "Screensaver"
        }

        OverrideTarget::Wallpaper => {
            "Wallpaper"
        }
    }
}

