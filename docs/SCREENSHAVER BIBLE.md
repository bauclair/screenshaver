# SCREENSHAVER BIBLE

## 🧭 Project Mantra

Observe → Instrument → Understand → Change → Verify

---

## 🎯 Philosophy

Screenshaver is built on five core principles:

- Simplicity over cleverness
- One module, one responsibility
- Explicit state flow (no hidden behavior)
- Deterministic execution
- Debug through observation, not speculation

---

## 🧱 System Architecture


---

## 📦 Module Contracts

---

## load_config

### Purpose
Load application configuration from disk.

### Inputs
- Path to `screenshaver.toml`

### Outputs
- `AppConfig`

### DO NOT
- Watch filesystem
- Perform runtime updates
- Interact with rendering or input systems

---

## poll_input

### Purpose
Detect user input events.

### Inputs
- SDL EventPump

### Outputs
- ActivitySignal { activity, quit }

### DO NOT
- Modify idle state
- Call renderer
- Perform timing logic

---

## track_idle

### Purpose
Track time since last user activity.

### Inputs
- ActivitySignal.activity

### Outputs
- Idle state (bool)

### DO NOT
- Read SDL events
- Decide rendering behavior
- Manage shaders

---

## resolve_state

### Purpose
Convert input + idle status into screen state.

### Inputs
- activity: bool
- idle: bool

### Outputs
- ScreenState { Active | Idle }

### DO NOT
- Access SDL
- Render graphics
- Track time directly

---

## render_frame

### Purpose
Render GLSL shaders or idle screen.

### Inputs
- ScreenState
- ShaderManager
- OpenGL context

### Outputs
- Frame to display

### DO NOT
- Handle input
- Track idle time
- Decide application state

---

## 🪵 Logging Standard

All modules must follow:

### Entry log


### Decision log


### Exit log


---

## 🧪 Debugging Methodology

1. Observe logs
2. Instrument suspicious module
3. Identify boundary violation
4. Fix behavior in correct module
5. Remove temporary logs

---

## 🚫 Global Rules

- No duplicated state across modules
- No cross-module responsibility leaks
- No hidden side effects
- No rendering logic outside render_frame
- No input handling outside poll_input

---

## 📈 Stability Rule

Once a module is verified correct:

> it is considered stable and should not be modified unless necessary

---

## 🧠 Future Expansion Areas

- multi-display support
- shader playlists
- configuration hot-reload
- Wayland support
- power/idle integration

These must NOT violate module contracts.

---

## 📌 Lessons Learned

- Execution flow must be explicit
- State must flow in one direction
- Debugging requires instrumentation, not guessing
- Architecture must be enforced, not assumed

