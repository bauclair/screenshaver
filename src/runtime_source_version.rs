//! Runtime shader-source preparation compatibility version.
//!
//! Increment this value whenever a change to shader preprocessing or runtime
//! source preparation can change the runtime shader source stored in the
//! Screenshaver database.
//!
//! A version change causes existing runtime-source artifacts to be regenerated
//! from their physical shader sources during reconciliation.

pub const RUNTIME_SOURCE_PREPARATION_VERSION: i64 = 2;
