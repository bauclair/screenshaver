I recommend treating log level and error classification as two separate numerical systems:

Log level controls how much information is written.
Event code identifies what subsystem produced the message and what kind of event occurred.

That prevents a shader-loading error from changing identity merely because the user selects a different verbosity level.

Proposed log levels

Each level includes everything from the lower levels.

Level 1 — Critical

Only events that prevent Screenshaver from starting, continuing, or shutting down safely.

Examples:

Root execution refused
Configuration cannot be loaded
No usable session backend
SDL initialization failure
Renderer cannot be created
Fatal internal state inconsistency

This should be the least verbose public-facing setting.

Level 2 — Errors

Level 1 plus recoverable failures.

Examples:

Shader compilation failed and another shader was selected
Texture generation failed and a fallback was used
Splash screen failed
Tray icon could not be created
Cache deletion failed
A requested shader was rejected
Level 3 — Warnings

Levels 1–2 plus suspicious or degraded behavior.

Examples:

Invalid configuration value replaced with a default
Unsupported shader construct rewritten
Requested palette not found
Missing optional asset
Preferred session backend unavailable
FPS override ignored
Shader file classified ambiguously

This would be a good default level for ordinary users.

Level 4 — Informational

Levels 1–3 plus normal major lifecycle events.

Examples:

Screenshaver started
Configuration loaded
Session backend selected
Splash displayed
Session became idle
Renderer engaged
Shader selected
Renderer disengaged
Screenshaver terminated cleanly

This gives a complete operational history without recording every internal action.

Level 5 — Debug

Levels 1–4 plus detailed decision-making information.

Examples:

Parsed configuration values
Mode and interval parsing diagnostics
Shader classification results
Texture specification selection
Selected primitive count
Shader preprocessing passes applied
Generated shader path
OpenGL version and renderer information
Fallback-selection reasoning
Session polling transitions

This replaces the current concept of a separate debug_log mode.

Level 6 — Trace

Everything.

Examples:

Function entry and exit where useful
Every file examined
Every shader candidate accepted or rejected
Every preprocessing transformation
Every uniform lookup
Every resource creation and destruction
Every session poll result
Every tray command poll
Timing and duration measurements
Per-frame events only where deliberately enabled

Level 6 becomes the maximum developer and power-user diagnostic log.

I would stop at six levels. More levels usually create distinctions that are difficult for users and developers to apply consistently.

Recommended event-code categories

Use four-digit numeric codes grouped by subsystem.

Range	Category
1000–1099	Process startup and shutdown
1100–1199	Security and privilege checks
1200–1299	Command-line parsing and command dispatch
2000–2099	Configuration loading and validation
2100–2199	Paths, files, directories, and cache
3000–3099	Singleton and process control
3100–3199	Tray icon and desktop integration
4000–4099	Session detection and idle monitoring
4100–4199	Wayland, X11, and backend selection
5000–5099	Shader discovery and selection
5100–5199	Shader classification
5200–5299	Shader parsing and preprocessing
5300–5399	Shader compilation and linking
5400–5499	Shader rejection and compatibility
6000–6099	Procedural texture selection
6100–6199	Texture generation
6200–6299	Palettes and texture parameters
7000–7099	SDL initialization and windows
7100–7199	OpenGL context and resources
7200–7299	Rendering lifecycle
7300–7399	Frame timing and FPS
8000–8099	Splash screen
8100–8199	Text overlays and subtitles
8200–8299	User messages and dialogs
9000–9099	Internal invariants and unexpected states
9900–9999	Reserved for temporary developer diagnostics

The ranges should identify the originating subsystem, not merely the severity.

For example:

1101  Root execution refused
2003  Configuration file could not be parsed
2007  Invalid configuration value replaced
4002  Session backend initialization failed
5004  Shader directory contained no usable shaders
5208  Existing GLSL version directive removed
5311  Fragment shader compilation failed
6105  Procedural texture generation failed
7001  SDL initialization failed
7204  Renderer engaged
8102  Subtitle texture creation failed
9001  Internal state invariant violated
Event-code structure

I recommend assigning each event one stable code:

pub const SECURITY_ROOT_REFUSED: u16 = 1101;
pub const CONFIG_LOAD_FAILED: u16 = 2003;
pub const SHADER_COMPILE_FAILED: u16 = 5311;

Do not derive event codes dynamically. They should be searchable in documentation, source code, bug reports, and user logs.

Severity versus verbosity

Each log entry should also carry a textual severity:

pub enum LogSeverity {
    Critical,
    Error,
    Warning,
    Information,
    Debug,
    Trace,
}

The configured numeric log level determines which severities are retained:

Severity	Minimum configured level
Critical	1
Error	2
Warning	3
Information	4
Debug	5
Trace	6

That produces simple monotonic behavior:

if configured_level >= event.minimum_level() {
    write_event(...);
}
Proposed log format

A consistent single-line format would make logs easier to read and parse:

2026-07-21T13:42:18.417-07:00 [L2] [ERROR] [E5311] [SHADER-COMPILER] Fragment shader compilation failed: ...

A normal lifecycle message:

2026-07-21T13:42:20.103-07:00 [L4] [INFO] [E7204] [RENDER] Renderer engaged

A detailed diagnostic:

2026-07-21T13:42:20.117-07:00 [L5] [DEBUG] [E5006] [SHADER] Selected random shader: poignant.glsl

At trace level:

2026-07-21T13:42:20.133-07:00 [L6] [TRACE] [E7308] [FRAME] Frame completed in 8.21 ms

I would use E to mean event, not necessarily error. This lets every important program event have a stable code.

Output policy

The runtime log should be the primary destination once its path is available and writable.

Use stderr only when:

The runtime log path cannot be determined.
The log file cannot be created or opened.
The logger itself fails.
The failure occurs before logging is initialized.
The user explicitly requested terminal output through a command-line operation.
A fatal error should be visible immediately in an attached terminal.

For a normal logged error, avoid duplicating it to stderr unless immediate visibility is useful.

A safe logger fallback sequence would be:

Attempt runtime log
        ↓ failure
Attempt stderr
        ↓ unavailable or detached
Terminate or continue as appropriate

The logger must never panic while reporting another error.

Configuration change

I recommend replacing:

debug_log = true

with:

log_level = 3

Suggested defaults:

1  Critical failures only
2  Errors
3  Warnings — recommended public default
4  Normal operational history
5  Debug diagnostics
6  Complete trace

For backward compatibility during one release cycle, debug_log = true could map to level 5, but since the program has not yet opened publicly, this is the ideal time to remove debug_log cleanly instead of retaining a compatibility path.

Important limitation on “every single program event”

Level 6 should be extremely detailed, but logging every rendered frame at 60–120 FPS would produce enormous logs and could alter rendering performance.

I recommend:

Record renderer startup, shutdown, timing summaries, dropped-frame warnings, and periodic performance statistics.
Do not record every frame by default.
Reserve per-frame logging for a separate temporary diagnostic option, such as:
trace_frames = true

That prevents the normal maximum level from generating gigabytes of data during a long-running screensaver session.

Recommended foundation

The refactor should therefore be based on:

Log levels:       1 through 6
Default level:    3
Public support:   levels 1 through 4
Developer use:    levels 5 and 6
Event codes:      four-digit subsystem codes
Primary output:   runtime log
Fallback output:  stderr

This gives Screenshaver a system that is understandable to users, useful in bug reports, and precise enough for developers without conflating severity, subsystem, and verbosity.
