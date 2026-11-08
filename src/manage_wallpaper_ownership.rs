//! Phase 3B-3E-10A: isolated wallpaper ownership transition model.
//!
//! This module has no I/O and is not connected to production renderers.
//! `NativeOwned` is a modeled state, not proof of actual exclusive rendering.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlasmaInstance {
    pub pid: u32,
    /// Linux /proc/<pid>/stat starttime, in clock ticks.
    pub start_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransactionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    LegacyOwned,
    NativePending {
        transaction: TransactionId,
        plasma: PlasmaInstance,
        legacy_quiesced: bool,
        native_granted: bool,
    },
    NativeOwned {
        transaction: TransactionId,
        plasma: PlasmaInstance,
    },
    RecoveryPending {
        transaction: TransactionId,
        plasma: PlasmaInstance,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    InvalidIdentity,
    InvalidTransition,
    WrongTransaction,
    WrongPlasmaInstance,
    LegacyNotQuiesced,
    NativeNotGranted,
}

#[derive(Debug, Clone)]
pub struct WallpaperOwnership {
    state: OwnershipState,
}

impl Default for WallpaperOwnership {
    fn default() -> Self { Self::new() }
}

impl WallpaperOwnership {
    pub fn new() -> Self { Self { state: OwnershipState::LegacyOwned } }

    pub fn state(&self) -> &OwnershipState { &self.state }

    pub fn begin_native_handoff(
        &mut self, transaction: TransactionId, plasma: PlasmaInstance,
    ) -> Result<(), OwnershipError> {
        if transaction.0.is_empty() || plasma.pid == 0 || plasma.start_ticks == 0 {
            return Err(OwnershipError::InvalidIdentity);
        }
        if !matches!(self.state, OwnershipState::LegacyOwned) {
            return Err(OwnershipError::InvalidTransition);
        }
        self.state = OwnershipState::NativePending {
            transaction, plasma, legacy_quiesced: false, native_granted: false,
        };
        Ok(())
    }

    pub fn legacy_quiesced(
        &mut self, transaction: &TransactionId,
    ) -> Result<(), OwnershipError> {
        match &mut self.state {
            OwnershipState::NativePending { transaction: current, legacy_quiesced, native_granted, .. } => {
                if current != transaction { return Err(OwnershipError::WrongTransaction); }
                if *native_granted { return Err(OwnershipError::InvalidTransition); }
                *legacy_quiesced = true;
                Ok(())
            }
            _ => Err(OwnershipError::InvalidTransition),
        }
    }

    pub fn native_granted(
        &mut self, transaction: &TransactionId, plasma: &PlasmaInstance,
    ) -> Result<(), OwnershipError> {
        match &mut self.state {
            OwnershipState::NativePending {
                transaction: current, plasma: expected, legacy_quiesced, native_granted,
            } => {
                if current != transaction { return Err(OwnershipError::WrongTransaction); }
                if expected != plasma { return Err(OwnershipError::WrongPlasmaInstance); }
                if !*legacy_quiesced { return Err(OwnershipError::LegacyNotQuiesced); }
                if *native_granted { return Err(OwnershipError::InvalidTransition); }
                *native_granted = true;
                Ok(())
            }
            _ => Err(OwnershipError::InvalidTransition),
        }
    }

    pub fn native_frame_confirmed(
        &mut self, transaction: &TransactionId, plasma: &PlasmaInstance,
    ) -> Result<(), OwnershipError> {
        match &self.state {
            OwnershipState::NativePending {
                transaction: current, plasma: expected, legacy_quiesced, native_granted,
            } => {
                if current != transaction { return Err(OwnershipError::WrongTransaction); }
                if expected != plasma { return Err(OwnershipError::WrongPlasmaInstance); }
                if !*legacy_quiesced { return Err(OwnershipError::LegacyNotQuiesced); }
                if !*native_granted { return Err(OwnershipError::NativeNotGranted); }
                self.state = OwnershipState::NativeOwned {
                    transaction: transaction.clone(), plasma: plasma.clone(),
                };
                Ok(())
            }
            _ => Err(OwnershipError::InvalidTransition),
        }
    }

    /// Model a native-to-native replacement while the legacy renderer remains
    /// quiesced. This is a state transition only: the production coordinator
    /// must separately verify the old process has exited, the old grant is
    /// absent, and the existing pause is still acknowledged under its lock.
    pub fn replace_native_generation(
        &mut self,
        old_transaction: &TransactionId,
        old_plasma: &PlasmaInstance,
        new_transaction: TransactionId,
        new_plasma: PlasmaInstance,
    ) -> Result<(), OwnershipError> {
        if new_transaction.0.is_empty() || new_plasma.pid == 0
            || new_plasma.start_ticks == 0 || new_plasma == *old_plasma
            || new_transaction == *old_transaction
        {
            return Err(OwnershipError::InvalidIdentity);
        }
        match &self.state {
            OwnershipState::NativeOwned { transaction, plasma }
                if transaction == old_transaction && plasma == old_plasma => {}
            OwnershipState::NativePending { transaction, plasma, legacy_quiesced: true, .. }
                if transaction == old_transaction && plasma == old_plasma => {}
            OwnershipState::NativeOwned { transaction, .. }
            | OwnershipState::NativePending { transaction, .. }
                if transaction != old_transaction => return Err(OwnershipError::WrongTransaction),
            OwnershipState::NativeOwned { .. }
            | OwnershipState::NativePending { .. } => return Err(OwnershipError::WrongPlasmaInstance),
            _ => return Err(OwnershipError::InvalidTransition),
        }
        self.state = OwnershipState::NativePending {
            transaction: new_transaction,
            plasma: new_plasma,
            legacy_quiesced: true,
            native_granted: false,
        };
        Ok(())
    }

    /// Request recovery. Does NOT authorize the legacy renderer to resume.
    pub fn begin_recovery(&mut self, transaction: &TransactionId) -> Result<(), OwnershipError> {
        match &self.state {
            OwnershipState::NativePending { transaction: current, plasma, .. }
            | OwnershipState::NativeOwned { transaction: current, plasma } => {
                if current != transaction { return Err(OwnershipError::WrongTransaction); }
                self.state = OwnershipState::RecoveryPending {
                    transaction: current.clone(), plasma: plasma.clone(),
                };
                Ok(())
            }
            _ => Err(OwnershipError::InvalidTransition),
        }
    }

    /// Only call after external verification that native presentation is quiescent.
    /// This function cannot independently verify that condition.
    pub fn native_revoked(
        &mut self, transaction: &TransactionId, plasma: &PlasmaInstance,
    ) -> Result<(), OwnershipError> {
        match &self.state {
            OwnershipState::RecoveryPending { transaction: current, plasma: expected } => {
                if current != transaction { return Err(OwnershipError::WrongTransaction); }
                if expected != plasma { return Err(OwnershipError::WrongPlasmaInstance); }
                self.state = OwnershipState::LegacyOwned;
                Ok(())
            }
            _ => Err(OwnershipError::InvalidTransition),
        }
    }
}

#[cfg(test)]
mod wallpaper_ownership_tests {
    use super::*;

    fn id(s: &str) -> TransactionId { TransactionId(s.into()) }
    fn plasma() -> PlasmaInstance { PlasmaInstance { pid: 1234, start_ticks: 5678 } }
    fn pending() -> WallpaperOwnership {
        let mut owner = WallpaperOwnership::new();
        owner.begin_native_handoff(id("tx-1"), plasma()).unwrap();
        owner
    }

    #[test]
    fn successful_handoff_and_verified_recovery() {
        let mut owner = pending();
        owner.legacy_quiesced(&id("tx-1")).unwrap();
        owner.native_granted(&id("tx-1"), &plasma()).unwrap();
        owner.native_frame_confirmed(&id("tx-1"), &plasma()).unwrap();
        assert!(matches!(owner.state(), OwnershipState::NativeOwned { .. }));
        owner.begin_recovery(&id("tx-1")).unwrap();
        assert!(matches!(owner.state(), OwnershipState::RecoveryPending { .. }));
        owner.native_revoked(&id("tx-1"), &plasma()).unwrap();
        assert_eq!(owner.state(), &OwnershipState::LegacyOwned);
    }

    #[test]
    fn stale_transaction_cannot_advance_or_recover() {
        let mut owner = pending();
        assert_eq!(owner.legacy_quiesced(&id("old")), Err(OwnershipError::WrongTransaction));
        assert_eq!(owner.begin_recovery(&id("old")), Err(OwnershipError::WrongTransaction));
        assert!(matches!(owner.state(), OwnershipState::NativePending { legacy_quiesced: false, .. }));
    }

    #[test]
    fn wrong_plasma_instance_is_rejected() {
        let mut owner = pending();
        owner.legacy_quiesced(&id("tx-1")).unwrap();
        let reused_pid = PlasmaInstance { pid: 1234, start_ticks: 9999 };
        assert_eq!(owner.native_granted(&id("tx-1"), &reused_pid), Err(OwnershipError::WrongPlasmaInstance));
        owner.native_granted(&id("tx-1"), &plasma()).unwrap();
        assert_eq!(owner.native_frame_confirmed(&id("tx-1"), &reused_pid), Err(OwnershipError::WrongPlasmaInstance));
    }

    #[test]
    fn grant_requires_legacy_quiescence() {
        let mut owner = pending();
        assert_eq!(owner.native_granted(&id("tx-1"), &plasma()), Err(OwnershipError::LegacyNotQuiesced));
        assert_eq!(owner.native_frame_confirmed(&id("tx-1"), &plasma()), Err(OwnershipError::LegacyNotQuiesced));
    }

    #[test]
    fn frame_confirmation_requires_grant() {
        let mut owner = pending();
        owner.legacy_quiesced(&id("tx-1")).unwrap();
        assert_eq!(owner.native_frame_confirmed(&id("tx-1"), &plasma()), Err(OwnershipError::NativeNotGranted));
    }

    #[test]
    fn interrupted_handoff_requires_explicit_revoke() {
        let mut owner = pending();
        owner.legacy_quiesced(&id("tx-1")).unwrap();
        owner.begin_recovery(&id("tx-1")).unwrap();
        assert_eq!(owner.begin_native_handoff(id("tx-2"), plasma()), Err(OwnershipError::InvalidTransition));
        assert!(matches!(owner.state(), OwnershipState::RecoveryPending { .. }));
        assert_eq!(owner.native_revoked(&id("old"), &plasma()), Err(OwnershipError::WrongTransaction));
        owner.native_revoked(&id("tx-1"), &plasma()).unwrap();
        assert_eq!(owner.state(), &OwnershipState::LegacyOwned);
    }

    #[test]
    fn invalid_identity_and_duplicate_grants_rejected() {
        let mut owner = WallpaperOwnership::new();
        assert_eq!(owner.begin_native_handoff(id(""), plasma()), Err(OwnershipError::InvalidIdentity));
        assert_eq!(owner.begin_native_handoff(id("tx"), PlasmaInstance { pid: 0, start_ticks: 1 }), Err(OwnershipError::InvalidIdentity));
        let mut owner = pending();
        assert_eq!(owner.begin_native_handoff(id("tx-2"), plasma()), Err(OwnershipError::InvalidTransition));
        owner.legacy_quiesced(&id("tx-1")).unwrap();
        owner.native_granted(&id("tx-1"), &plasma()).unwrap();
        assert_eq!(owner.native_granted(&id("tx-1"), &plasma()), Err(OwnershipError::InvalidTransition));
    }
}
