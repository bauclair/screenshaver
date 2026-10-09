//! Stage 5A-4: manufacturer-neutral OpenRGB device selection contract.
//!
//! This module is hardware-independent. The OpenRGB transport supplies the
//! discovered descriptors; no connection, mode change, or LED write occurs here.
//! Never infer a device's identity from its list position alone. Before any
//! write, the guarded session must revalidate identity and topology and persist
//! the original lighting state for verified restoration.

#[derive(Clone, Debug)]
pub struct AmbientDeviceDescriptor {
    /// OpenRGB controller identifier for this discovery snapshot.
    pub controller_id: u32,
    /// Unmodified, display-only name reported by OpenRGB.
    pub display_name: String,
    /// May be empty on devices without serial-number support.
    pub serial: String,
    pub led_count: usize,
    /// Optional device-reported matrix; no assumed dimensions.
    pub matrix: Option<crate::manage_ambient_lighting::LedMatrix>,
    /// Names as reported by OpenRGB; used to select supported modes.
    pub modes: Vec<String>,
    pub active_mode: usize,
}

#[derive(Clone, Debug)]
pub struct AmbientDeviceSelection {
    pub controller_id: u32,
    pub display_name: String,
    pub serial: String,
    pub led_count: usize,
    pub matrix: Option<crate::manage_ambient_lighting::LedMatrix>,
    pub original_mode: String,
    pub direct_mode: String,
}

/// Validate a user-selected device, without writing to it. An ambiguous name
/// is never sufficient to select a device; the controller ID must be explicit.
/// A device without a matrix is not eligible for spatial ambient lighting.
pub fn select_ambient_device(
    devices: &[AmbientDeviceDescriptor],
    selected_controller_id: u32,
) -> Result<AmbientDeviceSelection, String> {
    let mut matches = devices.iter().filter(|d| d.controller_id == selected_controller_id);
    let device = matches.next().ok_or("Selected OpenRGB controller not found")?;
    if matches.next().is_some() {
        return Err("Duplicate OpenRGB controller identifier".into());
    }
    if device.display_name.trim().is_empty() {
        return Err("OpenRGB controller has no display name".into());
    }
    if device.led_count == 0 || device.led_count > u16::MAX as usize {
        return Err("OpenRGB LED count is outside supported limits".into());
    }
    let matrix = device.matrix.as_ref().ok_or("Selected device has no spatial LED matrix")?;
    matrix.validate()?;
    if matrix.led_count != device.led_count {
        return Err("OpenRGB LED count and matrix topology disagree".into());
    }
    let original_mode = device.modes.get(device.active_mode)
        .ok_or("OpenRGB active mode index is invalid")?;
    let direct_mode = device.modes.iter().find(|mode| mode.eq_ignore_ascii_case("direct"))
        .ok_or("Selected OpenRGB controller does not support Direct mode")?;
    // A device already in Direct mode has no trustworthy pre-acquisition
    // lighting baseline. Do not take ownership or invent a restore target.
    if original_mode.eq_ignore_ascii_case("direct") {
        return Err("OpenRGB device is already in Direct mode; select a normal lighting mode before enabling ambient lighting".into());
    }
    Ok(AmbientDeviceSelection {
        controller_id: device.controller_id,
        display_name: device.display_name.clone(),
        serial: device.serial.clone(),
        led_count: device.led_count,
        matrix: Some(matrix.clone()),
        original_mode: original_mode.clone(),
        direct_mode: direct_mode.clone(),
    })
}

fn same_matrix(
    a: Option<&crate::manage_ambient_lighting::LedMatrix>,
    b: Option<&crate::manage_ambient_lighting::LedMatrix>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.columns == b.columns && a.rows == b.rows
            && a.led_count == b.led_count && a.indices == b.indices,
        (None, None) => true,
        _ => false,
    }
}

/// Reject a changed device before acquiring or restoring control. Controller
/// IDs alone are not stable identity; a missing serial requires conservative
/// matching of the remaining available metadata and topology.
pub fn verify_ambient_device_identity(
    selected: &AmbientDeviceSelection,
    observed: &AmbientDeviceDescriptor,
) -> Result<(), String> {
    if selected.controller_id != observed.controller_id
        || selected.display_name != observed.display_name
        || selected.serial != observed.serial
        || selected.led_count != observed.led_count
        || !same_matrix(selected.matrix.as_ref(), observed.matrix.as_ref())
    {
        return Err("OpenRGB device identity or LED topology changed".into());
    }
    // The pre-acquisition baseline must still be active when the session
    // revalidates the device under the exclusive ownership lock.
    if observed.modes.get(observed.active_mode).map(String::as_str)
        != Some(selected.original_mode.as_str())
    {
        return Err("OpenRGB active lighting mode changed before acquisition".into());
    }
    if !observed.modes.iter().any(|mode| mode == &selected.direct_mode) {
        return Err("OpenRGB Direct mode is no longer available".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device() -> AmbientDeviceDescriptor {
        AmbientDeviceDescriptor {
            controller_id: 7,
            display_name: "Example RGB Keyboard".into(),
            serial: "sample-serial".into(),
            led_count: 4,
            matrix: Some(crate::manage_ambient_lighting::LedMatrix {
                columns: 2, rows: 2, indices: vec![0, 1, 2, 3], led_count: 4,
            }),
            modes: vec!["Wave".into(), "Direct".into()],
            active_mode: 0,
        }
    }
    #[test]
    fn selects_discovered_topology_and_preserves_original_mode() {
        let d = device();
        let selection = select_ambient_device(&[d.clone()], 7).unwrap();
        assert_eq!(selection.led_count, 4);
        assert_eq!(selection.original_mode, "Wave");
        verify_ambient_device_identity(&selection, &d).unwrap();
    }
    #[test]
    fn rejects_missing_or_ambiguous_controller() {
        let d = device();
        assert!(select_ambient_device(&[d.clone()], 9).is_err());
        assert!(select_ambient_device(&[d.clone(), d], 7).is_err());
    }
    #[test]
    fn rejects_unsupported_topology_or_direct_mode() {
        let mut d = device();
        d.matrix = None;
        assert!(select_ambient_device(&[d.clone()], 7).is_err());
        d.matrix = device().matrix;
        d.modes.pop();
        assert!(select_ambient_device(&[d], 7).is_err());
    }
    #[test]
    fn rejects_device_already_in_direct_mode() {
        let mut d = device();
        d.active_mode = 1;
        assert!(select_ambient_device(&[d], 7).is_err());
    }
    #[test]
    fn rejects_active_mode_changed_after_selection() {
        let d = device();
        let selection = select_ambient_device(&[d.clone()], 7).unwrap();
        let mut changed = d;
        changed.active_mode = 1;
        assert!(verify_ambient_device_identity(&selection, &changed).is_err());
    }
    #[test]
    fn rejects_identity_or_topology_changes() {
        let d = device();
        let selection = select_ambient_device(&[d.clone()], 7).unwrap();
        let mut changed = d;
        changed.serial = "another-serial".into();
        assert!(verify_ambient_device_identity(&selection, &changed).is_err());
        changed.serial = "sample-serial".into();
        changed.matrix.as_mut().unwrap().indices.swap(0, 1);
        assert!(verify_ambient_device_identity(&selection, &changed).is_err());
    }
}
