//! Hardware-independent ownership and verified-restoration contract for ambient lighting.
//!
//! No OpenRGB calls are made here. The device coordinator must persist the
//! recovery record before acquisition, and confirm hardware restoration before
//! acknowledging it here. A renderer callback must never do network I/O.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AmbientSourceKind {
    Screensaver,
    Wallpaper,
    Windowshader,
    ControlCenterPreview,
    NativeLockScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AmbientSourceId {
    pub kind: AmbientSourceKind,
    pub instance: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmbientFrameSource {
    pub source: AmbientSourceId,
    pub generation: u64,
    /// Framebuffer in the submitting renderer's current GL context.
    pub framebuffer: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmbientTransition {
    None,
    /// Requires durable recovery state and verified device identity first.
    Acquire,
    /// Valid only for a direct handoff with no intervening rendering stop.
    Handoff,
    /// Restore and verify the original device lighting before acknowledging.
    RestoreOriginalLighting,
    /// Do not acquire: a previous restoration remains unverified.
    AwaitRestoration,
}

#[derive(Debug, Default)]
pub struct AmbientSourceCoordinator {
    active: Option<AmbientSourceId>,
    generation: u64,
    restoration_pending: bool,
}

impl AmbientSourceCoordinator {
    pub fn new() -> Self { Self::default() }

    pub fn active(&self) -> Option<AmbientSourceId> { self.active }

    pub fn generation(&self) -> u64 { self.generation }

    pub fn restoration_pending(&self) -> bool { self.restoration_pending }

    /// Called only when a renderer is genuinely producing eligible frames.
    /// The returned action must be completed by the device coordinator before
    /// frames are sent to OpenRGB. `Acquire` does not itself acquire hardware.
    pub fn activate(&mut self, source: AmbientSourceId) -> (u64, AmbientTransition) {
        if self.restoration_pending {
            return (self.generation, AmbientTransition::AwaitRestoration);
        }
        if self.active == Some(source) {
            return (self.generation, AmbientTransition::None);
        }
        let transition = if self.active.is_some() {
            AmbientTransition::Handoff
        } else {
            AmbientTransition::Acquire
        };
        self.generation = self.generation.wrapping_add(1);
        self.active = Some(source);
        (self.generation, transition)
    }

    /// Stop accepting frames immediately. A stale source cannot stop a newer
    /// source. A restoration request remains pending until explicitly verified.
    pub fn suspend(&mut self, source: AmbientSourceId) -> AmbientTransition {
        if self.active != Some(source) {
            return AmbientTransition::None;
        }
        self.active = None;
        self.generation = self.generation.wrapping_add(1);
        self.restoration_pending = true;
        AmbientTransition::RestoreOriginalLighting
    }

    /// Only call after the guarded device manager has verified restoration and
    /// safely released ownership. Never call merely because a request was sent.
    pub fn acknowledge_verified_restoration(&mut self) -> bool {
        if !self.restoration_pending || self.active.is_some() {
            return false;
        }
        self.restoration_pending = false;
        true
    }

    /// If a recovery record was found on startup, block new acquisitions until
    /// the guarded recovery implementation has restored the original lighting.
    pub fn require_recovery(&mut self) {
        self.active = None;
        self.generation = self.generation.wrapping_add(1);
        self.restoration_pending = true;
    }

    pub fn accepts(&self, frame: &AmbientFrameSource) -> bool {
        !self.restoration_pending
            && self.active == Some(frame.source)
            && self.generation == frame.generation
            && frame.width > 0
            && frame.height > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(kind: AmbientSourceKind) -> AmbientSourceId {
        AmbientSourceId { kind, instance: 1 }
    }

    #[test]
    fn stopping_blocks_reacquisition_until_verified() {
        let mut c = AmbientSourceCoordinator::new();
        let s = id(AmbientSourceKind::Windowshader);
        let (generation, action) = c.activate(s);
        assert_eq!(action, AmbientTransition::Acquire);
        let frame = AmbientFrameSource { source: s, generation, framebuffer: 3, width: 960, height: 540 };
        assert!(c.accepts(&frame));
        assert_eq!(c.suspend(s), AmbientTransition::RestoreOriginalLighting);
        assert!(!c.accepts(&frame));
        assert_eq!(c.activate(s).1, AmbientTransition::AwaitRestoration);
        assert!(c.restoration_pending());
        assert!(c.acknowledge_verified_restoration());
        assert!(!c.acknowledge_verified_restoration());
        assert_eq!(c.activate(s).1, AmbientTransition::Acquire);
    }

    #[test]
    fn direct_handoff_invalidates_previous_generation() {
        let mut c = AmbientSourceCoordinator::new();
        let wallpaper = id(AmbientSourceKind::Wallpaper);
        let screensaver = id(AmbientSourceKind::Screensaver);
        let (old, _) = c.activate(wallpaper);
        let (new, action) = c.activate(screensaver);
        assert_eq!(action, AmbientTransition::Handoff);
        assert_ne!(old, new);
        assert_eq!(c.suspend(wallpaper), AmbientTransition::None);
        assert_eq!(c.active(), Some(screensaver));
        assert_eq!(c.suspend(screensaver), AmbientTransition::RestoreOriginalLighting);
    }

    #[test]
    fn startup_recovery_blocks_all_frames() {
        let mut c = AmbientSourceCoordinator::new();
        c.require_recovery();
        assert_eq!(c.activate(id(AmbientSourceKind::Screensaver)).1,
                   AmbientTransition::AwaitRestoration);
        assert!(c.acknowledge_verified_restoration());
        assert_eq!(c.activate(id(AmbientSourceKind::Screensaver)).1,
                   AmbientTransition::Acquire);
    }

    #[test]
    fn no_shader_means_no_acquisition() {
        let c = AmbientSourceCoordinator::new();
        assert_eq!(c.active(), None);
        assert!(!c.restoration_pending());
    }
}
