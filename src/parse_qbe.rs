//! Semantic model and contextual rules for Policy List Query By Example.
//!
//! This module deliberately does not access SQLite and does not draw egui
//! controls.  It is the single authority for:
//! - queryable fields,
//! - context-valid operators,
//! - value-editor kinds,
//! - simple/compound query classification,
//! - blank/default QBE state,
//! - structural validation.
//!
//! SQL construction will be added here after the UI semantics are approved.
//! query_database.rs will remain responsible for executing the resulting
//! parameterized database query.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeField {
    PolicyName,
    ShaderFilename,
    ShaderType,
    PolicyTarget,
    Status,
    Texture,
    Palette,
    RenderedFps,
    AnimationSpeed,
    RenderScale,
    AntiAliasing,
    Dithering,
    ColorPrecision,
    BloomMode,
}


impl QbeField {

    pub const ALL: &'static [Self] = &[
        Self::PolicyName,
        Self::ShaderFilename,
        Self::ShaderType,
        Self::PolicyTarget,
        Self::Status,
        Self::Texture,
        Self::Palette,
        Self::RenderedFps,
        Self::AnimationSpeed,
        Self::RenderScale,
        Self::AntiAliasing,
        Self::Dithering,
        Self::ColorPrecision,
        Self::BloomMode,
    ];


    pub const fn label(
        self,
    ) -> &'static str {

        match self {
            Self::PolicyName =>
                "Policy Name",

            Self::ShaderFilename =>
                "Shader Filename",

            Self::ShaderType =>
                "Shader Type",

            Self::PolicyTarget =>
                "Policy Target",

            Self::Status =>
                "Status",

            Self::Texture =>
                "Texture",

            Self::Palette =>
                "Palette",

            Self::RenderedFps =>
                "Rendered FPS",

            Self::AnimationSpeed =>
                "Animation Speed",

            Self::RenderScale =>
                "Render Scale",

            Self::AntiAliasing =>
                "Anti-Aliasing",

            Self::Dithering =>
                "Dithering",

            Self::ColorPrecision =>
                "Color Precision",

            Self::BloomMode =>
                "Bloom Mode",
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeOperator {
    Is,
    Eq,
    Ne,
    Like,
    NotLike,
    Lt,
    Le,
    Gt,
    Ge,
}


impl QbeOperator {

    pub const fn label(
        self,
    ) -> &'static str {

        match self {
            Self::Is =>
                "is",

            Self::Eq =>
                "eq",

            Self::Ne =>
                "ne",

            Self::Like =>
                "like",

            Self::NotLike =>
                "not like",

            Self::Lt =>
                "lt",

            Self::Le =>
                "le",

            Self::Gt =>
                "gt",

            Self::Ge =>
                "ge",
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeConditional {
    And,
    Or,
}


impl QbeConditional {

    pub const ALL: &'static [Self] = &[
        Self::And,
        Self::Or,
    ];


    pub const fn label(
        self,
    ) -> &'static str {

        match self {
            Self::And =>
                "AND",

            Self::Or =>
                "OR",
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeQueryKind {
    Empty,
    Simple,
    Compound,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeValueKind {
    Boolean,
    Text,
    Integer,
    Decimal,
    ShaderType,
    PolicyTarget,
    TextureName,
    PaletteName,
    Status,
    AntiAliasing,
    Dithering,
    ColorPrecision,
    BloomMode,
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QbeClause {
    pub field: Option<QbeField>,
    pub operator: Option<QbeOperator>,
    pub value: String,
}


impl QbeClause {

    pub fn clear(
        &mut self,
    ) {

        *self =
            Self::default();
    }


    pub fn is_blank(
        &self,
    ) -> bool {

        self.field.is_none()
            && self.operator.is_none()
            && self.value.trim().is_empty()
    }


    pub fn is_complete(
        &self,
    ) -> bool {

        let (
            Some(field),
            Some(operator),
        ) = (
            self.field,
            self.operator,
        )
        else {
            return false;
        };


        operator_is_valid(
            field,
            operator,
        )
            && !self.value.trim().is_empty()
    }


    pub fn normalize_after_field_change(
        &mut self,
    ) {

        let Some(field) =
            self.field
        else {
            self.operator =
                None;

            self.value.clear();

            return;
        };


        if let Some(operator) =
            self.operator
        {
            if !operator_is_valid(
                field,
                operator,
            ) {
                self.operator =
                    None;

                self.value.clear();
            }
        }
    }


    pub fn normalize_after_operator_change(
        &mut self,
    ) {

        let (
            Some(field),
            Some(operator),
        ) = (
            self.field,
            self.operator,
        )
        else {
            self.value.clear();

            return;
        };


        if !operator_is_valid(
            field,
            operator,
        ) {
            self.operator =
                None;

            self.value.clear();
        }
    }
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QbeState {
    pub first: QbeClause,
    pub conditional: Option<QbeConditional>,
    pub second: QbeClause,
}


impl QbeState {

    pub fn clear(
        &mut self,
    ) {

        *self =
            Self::default();
    }


    pub fn normalize(
        &mut self,
    ) {

        self.first
            .normalize_after_field_change();

        self.first
            .normalize_after_operator_change();


        if !self.first.is_complete() {
            self.conditional =
                None;

            self.second.clear();

            return;
        }


        if self.conditional.is_none() {
            self.second.clear();

            return;
        }


        self.second
            .normalize_after_field_change();

        self.second
            .normalize_after_operator_change();
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QbeValidationError {
    FirstFieldMissing,
    FirstOperatorMissing,
    FirstValueMissing,
    FirstOperatorInvalid,
    SecondFieldMissing,
    SecondOperatorMissing,
    SecondValueMissing,
    SecondOperatorInvalid,
}


impl QbeValidationError {

    pub const fn message(
        self,
    ) -> &'static str {

        match self {
            Self::FirstFieldMissing =>
                "The first QBE item is blank.",

            Self::FirstOperatorMissing =>
                "The first QBE operator is blank.",

            Self::FirstValueMissing =>
                "The first QBE value is blank.",

            Self::FirstOperatorInvalid =>
                "The first QBE operator is not valid for the selected item.",

            Self::SecondFieldMissing =>
                "The second QBE item is blank.",

            Self::SecondOperatorMissing =>
                "The second QBE operator is blank.",

            Self::SecondValueMissing =>
                "The second QBE value is blank.",

            Self::SecondOperatorInvalid =>
                "The second QBE operator is not valid for the selected item.",
        }
    }
}


const TEXT_OPERATORS: &[QbeOperator] = &[
    QbeOperator::Eq,
    QbeOperator::Ne,
    QbeOperator::Like,
    QbeOperator::NotLike,
];

const NUMERIC_OPERATORS: &[QbeOperator] = &[
    QbeOperator::Eq,
    QbeOperator::Ne,
    QbeOperator::Lt,
    QbeOperator::Le,
    QbeOperator::Gt,
    QbeOperator::Ge,
];

const ENUM_OPERATORS: &[QbeOperator] = &[
    QbeOperator::Eq,
    QbeOperator::Ne,
];

const CONTEXTUAL_RESOURCE_OPERATORS: &[QbeOperator] = &[
    QbeOperator::Is,
    QbeOperator::Eq,
    QbeOperator::Ne,
    QbeOperator::Like,
    QbeOperator::NotLike,
];


pub const fn operators_for(
    field: QbeField,
) -> &'static [QbeOperator] {

    match field {
        QbeField::PolicyName
        | QbeField::ShaderFilename =>
            TEXT_OPERATORS,

        QbeField::RenderedFps
        | QbeField::AnimationSpeed
        | QbeField::RenderScale =>
            NUMERIC_OPERATORS,

        QbeField::Texture
        | QbeField::Palette =>
            CONTEXTUAL_RESOURCE_OPERATORS,

        QbeField::ShaderType
        | QbeField::PolicyTarget
        | QbeField::Status
        | QbeField::AntiAliasing
        | QbeField::Dithering
        | QbeField::ColorPrecision
        | QbeField::BloomMode =>
            ENUM_OPERATORS,
    }
}


pub fn operator_is_valid(
    field: QbeField,
    operator: QbeOperator,
) -> bool {

    operators_for(
        field
    )
    .contains(
        &operator
    )
}


pub const fn value_kind_for(
    field: QbeField,
    operator: QbeOperator,
) -> QbeValueKind {

    match (
        field,
        operator,
    ) {
        (
            QbeField::Texture
            | QbeField::Palette,
            QbeOperator::Is,
        ) => {
            QbeValueKind::Boolean
        }

        (
            QbeField::Texture,
            _,
        ) => {
            QbeValueKind::TextureName
        }

        (
            QbeField::Palette,
            _,
        ) => {
            QbeValueKind::PaletteName
        }

        (
            QbeField::ShaderType,
            _,
        ) => {
            QbeValueKind::ShaderType
        }

        (
            QbeField::PolicyTarget,
            _,
        ) => {
            QbeValueKind::PolicyTarget
        }

        (
            QbeField::Status,
            _,
        ) => {
            QbeValueKind::Status
        }

        (
            QbeField::RenderedFps,
            _,
        ) => {
            QbeValueKind::Integer
        }

        (
            QbeField::AnimationSpeed
            | QbeField::RenderScale,
            _,
        ) => {
            QbeValueKind::Decimal
        }

        (
            QbeField::AntiAliasing,
            _,
        ) => {
            QbeValueKind::AntiAliasing
        }

        (
            QbeField::Dithering,
            _,
        ) => {
            QbeValueKind::Dithering
        }

        (
            QbeField::ColorPrecision,
            _,
        ) => {
            QbeValueKind::ColorPrecision
        }

        (
            QbeField::BloomMode,
            _,
        ) => {
            QbeValueKind::BloomMode
        }

        (
            QbeField::PolicyName
            | QbeField::ShaderFilename,
            _,
        ) => {
            QbeValueKind::Text
        }
    }
}


pub fn query_kind(
    state: &QbeState,
) -> QbeQueryKind {

    if state.first.is_blank()
        && state.conditional.is_none()
        && state.second.is_blank()
    {
        QbeQueryKind::Empty
    } else if state.conditional.is_some() {
        QbeQueryKind::Compound
    } else {
        QbeQueryKind::Simple
    }
}


pub fn validate(
    state: &QbeState,
) -> Result<QbeQueryKind, QbeValidationError> {

    if state.first.field.is_none() {
        return Err(
            QbeValidationError::FirstFieldMissing
        );
    }


    let first_field =
        state.first.field
            .expect(
                "first field checked above"
            );


    if state.first.operator.is_none() {
        return Err(
            QbeValidationError::FirstOperatorMissing
        );
    }


    let first_operator =
        state.first.operator
            .expect(
                "first operator checked above"
            );


    if !operator_is_valid(
        first_field,
        first_operator,
    ) {
        return Err(
            QbeValidationError::FirstOperatorInvalid
        );
    }


    if state.first.value.trim().is_empty() {
        return Err(
            QbeValidationError::FirstValueMissing
        );
    }


    if state.conditional.is_none() {
        return Ok(
            QbeQueryKind::Simple
        );
    }


    if state.second.field.is_none() {
        return Err(
            QbeValidationError::SecondFieldMissing
        );
    }


    let second_field =
        state.second.field
            .expect(
                "second field checked above"
            );


    if state.second.operator.is_none() {
        return Err(
            QbeValidationError::SecondOperatorMissing
        );
    }


    let second_operator =
        state.second.operator
            .expect(
                "second operator checked above"
            );


    if !operator_is_valid(
        second_field,
        second_operator,
    ) {
        return Err(
            QbeValidationError::SecondOperatorInvalid
        );
    }


    if state.second.value.trim().is_empty() {
        return Err(
            QbeValidationError::SecondValueMissing
        );
    }


    Ok(
        QbeQueryKind::Compound
    )
}

// ============================================================
// PARAMETERIZED SQL CONSTRUCTION
// ============================================================

#[derive(Clone, Debug, PartialEq)]
pub enum QbeSqlParameter {
    Text(String),
    Integer(i64),
    Real(f64),
}


#[derive(Clone, Debug, PartialEq)]
pub struct QbeSql {
    /// SQL boolean expression without the leading WHERE keyword.
    ///
    /// query_database.rs owns the canonical SELECT/JOIN statement and may
    /// append `WHERE {where_clause}` when this string is non-empty.
    pub where_clause: String,

    /// Positional bind values in the exact order of the `?` placeholders
    /// appearing in where_clause.
    pub parameters: Vec<QbeSqlParameter>,

    pub kind: QbeQueryKind,
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QbeParseError {
    Validation(QbeValidationError),
    InvalidBoolean(String),
    InvalidInteger(String),
    InvalidDecimal(String),
    InvalidShaderType(String),
    InvalidPolicyTarget(String),
    InvalidStatus(String),
    InvalidAntiAliasing(String),
    InvalidDithering(String),
    InvalidColorPrecision(String),
    InvalidBloomMode(String),
}


impl QbeParseError {

    pub fn message(
        &self,
    ) -> String {

        match self {
            Self::Validation(error) =>
                error.message().to_string(),

            Self::InvalidBoolean(value) =>
                format!(
                    "QBE boolean value '{}' must be true or false.",
                    value,
                ),

            Self::InvalidInteger(value) =>
                format!(
                    "QBE value '{}' is not a valid integer.",
                    value,
                ),

            Self::InvalidDecimal(value) =>
                format!(
                    "QBE value '{}' is not a valid decimal number.",
                    value,
                ),

            Self::InvalidShaderType(value) =>
                format!(
                    "Unknown Shader Type '{}'.",
                    value,
                ),

            Self::InvalidPolicyTarget(value) =>
                format!(
                    "Unknown Policy Target '{}'.",
                    value,
                ),

            Self::InvalidStatus(value) =>
                format!(
                    "Unknown shader Status '{}'.",
                    value,
                ),

            Self::InvalidAntiAliasing(value) =>
                format!(
                    "Unknown Anti-Aliasing value '{}'.",
                    value,
                ),

            Self::InvalidDithering(value) =>
                format!(
                    "Unknown Dithering value '{}'.",
                    value,
                ),

            Self::InvalidColorPrecision(value) =>
                format!(
                    "Unknown Color Precision value '{}'.",
                    value,
                ),

            Self::InvalidBloomMode(value) =>
                format!(
                    "Unknown Bloom Mode value '{}'.",
                    value,
                ),
        }
    }
}


impl From<QbeValidationError> for QbeParseError {

    fn from(
        value: QbeValidationError,
    ) -> Self {

        Self::Validation(value)
    }
}


pub fn build_sql(
    state: &QbeState,
) -> Result<QbeSql, QbeParseError> {

    let kind =
        validate(
            state
        )?;


    let first =
        build_clause_sql(
            &state.first
        )?;


    let mut where_clause =
        first.sql;

    let mut parameters =
        first.parameters;


    if kind == QbeQueryKind::Compound {
        let conditional =
            state.conditional
                .expect(
                    "compound QBE must have a conditional"
                );

        let second =
            build_clause_sql(
                &state.second
            )?;


        where_clause =
            format!(
                "({}) {} ({})",
                where_clause,
                conditional.label(),
                second.sql,
            );

        parameters.extend(
            second.parameters
        );
    }


    Ok(
        QbeSql {
            where_clause,
            parameters,
            kind,
        }
    )
}


#[derive(Clone, Debug)]
struct ClauseSql {
    sql: String,
    parameters: Vec<QbeSqlParameter>,
}


fn build_clause_sql(
    clause: &QbeClause,
) -> Result<ClauseSql, QbeParseError> {

    let field =
        clause.field
            .ok_or(
                QbeValidationError::FirstFieldMissing
            )?;

    let operator =
        clause.operator
            .ok_or(
                QbeValidationError::FirstOperatorMissing
            )?;

    let value =
        clause.value.trim();


    match field {
        QbeField::PolicyName => {
            build_text_column_clause(
                "p.policy_name",
                operator,
                value,
            )
        }


        QbeField::ShaderFilename => {
            build_text_column_clause(
                "s.filename",
                operator,
                value,
            )
        }


        QbeField::ShaderType => {
            let canonical =
                canonical_shader_type(
                    value
                )?;

            build_enum_column_clause(
                "s.shader_type",
                operator,
                canonical,
            )
        }


        QbeField::PolicyTarget => {
            let canonical =
                canonical_policy_target(
                    value
                )?;

            build_enum_column_clause(
                "p.policy_target",
                operator,
                canonical,
            )
        }


        QbeField::Status => {
            build_status_clause(
                operator,
                value,
            )
        }


        QbeField::Texture => {
            build_texture_clause(
                operator,
                value,
            )
        }


        QbeField::Palette => {
            build_palette_clause(
                operator,
                value,
            )
        }


        QbeField::RenderedFps => {
            build_integer_column_clause(
                "p.rendered_fps",
                operator,
                value,
            )
        }


        QbeField::AnimationSpeed => {
            build_real_column_clause(
                "p.animation_speed",
                operator,
                value,
            )
        }


        QbeField::RenderScale => {
            build_real_column_clause(
                "p.render_scale",
                operator,
                value,
            )
        }


        QbeField::AntiAliasing => {
            let canonical =
                canonical_anti_aliasing(
                    value
                )?;

            build_enum_column_clause(
                "p.anti_aliasing",
                operator,
                canonical,
            )
        }


        QbeField::Dithering => {
            let canonical =
                canonical_dithering(
                    value
                )?;

            build_enum_column_clause(
                "p.dithering",
                operator,
                canonical,
            )
        }


        QbeField::ColorPrecision => {
            let canonical =
                canonical_color_precision(
                    value
                )?;

            build_enum_column_clause(
                "p.color_precision",
                operator,
                canonical,
            )
        }


        QbeField::BloomMode => {
            let canonical =
                canonical_bloom_mode(
                    value
                )?;

            build_enum_column_clause(
                "p.bloom_mode",
                operator,
                canonical,
            )
        }
    }
}


fn build_text_column_clause(
    column: &str,
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    let sql_operator =
        match operator {
            QbeOperator::Eq =>
                "=",

            QbeOperator::Ne =>
                "<>",

            QbeOperator::Like =>
                "LIKE",

            QbeOperator::NotLike =>
                "NOT LIKE",

            _ => {
                return Err(
                    QbeParseError::Validation(
                        QbeValidationError::FirstOperatorInvalid
                    )
                );
            }
        };


    let parameter =
        if matches!(
            operator,
            QbeOperator::Like
                | QbeOperator::NotLike
        ) {
            format!(
                "%{}%",
                value,
            )
        } else {
            value.to_string()
        };


    Ok(
        ClauseSql {
            sql:
                format!(
                    "LOWER({}) {} LOWER(?)",
                    column,
                    sql_operator,
                ),

            parameters:
                vec![
                    QbeSqlParameter::Text(
                        parameter
                    )
                ],
        }
    )
}


fn build_enum_column_clause(
    column: &str,
    operator: QbeOperator,
    canonical_value: String,
) -> Result<ClauseSql, QbeParseError> {

    let sql_operator =
        match operator {
            QbeOperator::Eq =>
                "=",

            QbeOperator::Ne =>
                "<>",

            _ => {
                return Err(
                    QbeParseError::Validation(
                        QbeValidationError::FirstOperatorInvalid
                    )
                );
            }
        };


    Ok(
        ClauseSql {
            sql:
                format!(
                    "LOWER(COALESCE({}, '')) {} LOWER(?)",
                    column,
                    sql_operator,
                ),

            parameters:
                vec![
                    QbeSqlParameter::Text(
                        canonical_value
                    )
                ],
        }
    )
}


fn build_integer_column_clause(
    column: &str,
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    let parsed =
        value.parse::<i64>()
            .map_err(
                |_| {
                    QbeParseError::InvalidInteger(
                        value.to_string()
                    )
                }
            )?;


    Ok(
        ClauseSql {
            sql:
                format!(
                    "{} {} ?",
                    column,
                    numeric_sql_operator(
                        operator
                    )?,
                ),

            parameters:
                vec![
                    QbeSqlParameter::Integer(
                        parsed
                    )
                ],
        }
    )
}


fn build_real_column_clause(
    column: &str,
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    let parsed =
        value.parse::<f64>()
            .map_err(
                |_| {
                    QbeParseError::InvalidDecimal(
                        value.to_string()
                    )
                }
            )?;


    if !parsed.is_finite() {
        return Err(
            QbeParseError::InvalidDecimal(
                value.to_string()
            )
        );
    }


    Ok(
        ClauseSql {
            sql:
                format!(
                    "{} {} ?",
                    column,
                    numeric_sql_operator(
                        operator
                    )?,
                ),

            parameters:
                vec![
                    QbeSqlParameter::Real(
                        parsed
                    )
                ],
        }
    )
}


fn numeric_sql_operator(
    operator: QbeOperator,
) -> Result<&'static str, QbeParseError> {

    match operator {
        QbeOperator::Eq =>
            Ok("="),

        QbeOperator::Ne =>
            Ok("<>"),

        QbeOperator::Lt =>
            Ok("<"),

        QbeOperator::Le =>
            Ok("<="),

        QbeOperator::Gt =>
            Ok(">"),

        QbeOperator::Ge =>
            Ok(">="),

        _ => {
            Err(
                QbeParseError::Validation(
                    QbeValidationError::FirstOperatorInvalid
                )
            )
        }
    }
}


fn build_texture_clause(
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    if operator == QbeOperator::Is {
        let enabled =
            parse_boolean(
                value
            )?;


        // Texture "is true/false" describes whether the underlying shader
        // actually consumes one or more texture channels.  It intentionally
        // does not inspect p.texture_mode, because NULL there means "inherit"
        // rather than "shader does not use textures."
        return Ok(
            ClauseSql {
                sql:
                    if enabled {
                        "COALESCE(s.channel_usage_mask, 0) <> 0"
                            .to_string()
                    } else {
                        "COALESCE(s.channel_usage_mask, 0) = 0"
                            .to_string()
                    },

                parameters:
                    Vec::new(),
            }
        );
    }


    build_text_column_clause(
        "COALESCE(p.texture_family, '')",
        operator,
        value,
    )
}


fn build_palette_clause(
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    if operator == QbeOperator::Is {
        let enabled =
            parse_boolean(
                value
            )?;


        return Ok(
            ClauseSql {
                sql:
                    if enabled {
                        "p.palette_mode IS NOT NULL"
                            .to_string()
                    } else {
                        "p.palette_mode IS NULL"
                            .to_string()
                    },

                parameters:
                    Vec::new(),
            }
        );
    }


    // A policy can store either a specific #rrggbb palette or the symbolic
    // mode "random".  Coalescing the two makes both values queryable through
    // one user-facing Palette field.
    build_text_column_clause(
        "COALESCE(p.palette_color, p.palette_mode, '')",
        operator,
        value,
    )
}


fn build_status_clause(
    operator: QbeOperator,
    value: &str,
) -> Result<ClauseSql, QbeParseError> {

    let predicate =
        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "ok"
            | "valid" => {
                "s.file_status = 'present' AND s.validation_status = 'valid'"
            }

            "rejected" => {
                "s.validation_status = 'rejected'"
            }

            "missing" => {
                "s.file_status = 'missing'"
            }

            "unreadable" => {
                "s.file_status = 'unreadable'"
            }

            _ => {
                return Err(
                    QbeParseError::InvalidStatus(
                        value.to_string()
                    )
                );
            }
        };


    let sql =
        match operator {
            QbeOperator::Eq => {
                format!(
                    "({})",
                    predicate,
                )
            }

            QbeOperator::Ne => {
                format!(
                    "NOT ({})",
                    predicate,
                )
            }

            _ => {
                return Err(
                    QbeParseError::Validation(
                        QbeValidationError::FirstOperatorInvalid
                    )
                );
            }
        };


    Ok(
        ClauseSql {
            sql,
            parameters:
                Vec::new(),
        }
    )
}


fn parse_boolean(
    value: &str,
) -> Result<bool, QbeParseError> {

    match value
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "true" =>
            Ok(true),

        "false" =>
            Ok(false),

        _ => {
            Err(
                QbeParseError::InvalidBoolean(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_shader_type(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "native"
        | "nativeglsl" =>
            Ok(
                "native".to_string()
            ),

        "isf" =>
            Ok(
                "isf".to_string()
            ),

        "shadertoy" =>
            Ok(
                "shadertoy".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidShaderType(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_policy_target(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "screensaver" =>
            Ok(
                "screensaver".to_string()
            ),

        "wallpaper" =>
            Ok(
                "wallpaper".to_string()
            ),

        "unassigned" =>
            Ok(
                "unassigned".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidPolicyTarget(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_anti_aliasing(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "off" =>
            Ok(
                "off".to_string()
            ),

        "fxaa" =>
            Ok(
                "fxaa".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidAntiAliasing(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_dithering(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "off" =>
            Ok(
                "off".to_string()
            ),

        "subtle" =>
            Ok(
                "subtle".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidDithering(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_color_precision(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "automatic"
        | "auto" =>
            Ok(
                "auto".to_string()
            ),

        "high"
        | "highprecision" =>
            Ok(
                "high".to_string()
            ),

        "standard"
        | "standardprecision" =>
            Ok(
                "standard".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidColorPrecision(
                    value.to_string()
                )
            )
        }
    }
}


fn canonical_bloom_mode(
    value: &str,
) -> Result<String, QbeParseError> {

    match compact_key(
        value
    )
    .as_str()
    {
        "off" =>
            Ok(
                "off".to_string()
            ),

        "highlight" =>
            Ok(
                "highlight".to_string()
            ),

        "audio" =>
            Ok(
                "audio".to_string()
            ),

        _ => {
            Err(
                QbeParseError::InvalidBloomMode(
                    value.to_string()
                )
            )
        }
    }
}


fn compact_key(
    value: &str,
) -> String {

    value
        .trim()
        .chars()
        .filter(
            |character| {
                !character.is_whitespace()
                    && *character != '-'
                    && *character != '_'
            }
        )
        .flat_map(
            |character| {
                character.to_lowercase()
            }
        )
        .collect()
}

