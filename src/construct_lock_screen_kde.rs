//! construct_lock_screen_kde.rs
//!
//! Constructs the KDE Plasma / KScreenLocker QML representation of the
//! Screenshaver lock-screen authentication widget.
//!
//! The visual parameters come from LockScreenWidgetConfig so the native
//! Screenshaver renderer and KDE presentation share the same authoritative
//! widget definition.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use crate::define_lock_screen_widget::LockScreenWidgetConfig;

const CHILD_COUNT: usize = 12;
const HALO_RING_COUNT: usize = 12;

fn qml_rgba(color: [f32; 4]) -> String {
    format!(
        "Qt.rgba({:.8}, {:.8}, {:.8}, {:.8})",
        color[0],
        color[1],
        color[2],
        color[3]
    )
}

/// Constructs a complete LockScreen.qml source string for KDE Plasma's
/// KScreenLocker greeter.
///
/// Authentication remains under KScreenLocker control through its injected
/// `authenticator` object. Screenshaver supplies presentation and the
/// text-free password-entry visualization.
pub fn construct_lock_screen_kde(
    config: &LockScreenWidgetConfig,
) -> String {
    let child_inactive_color =
        qml_rgba(config.child_inactive_color);

    let child_active_color =
        qml_rgba(config.child_active_color);

    let child_error_color =
        qml_rgba(config.child_error_color);

    let background_color =
        qml_rgba(config.background_color);

    let halo_color =
        qml_rgba(config.halo_color);

    let fade_ms =
        config.child_active_fade_time
            .as_millis();

    let failure_ms =
        config.authentication_failure_duration
            .as_millis();

    let mut qml =
        String::new();

    writeln!(qml, "import QtQuick").unwrap();
    writeln!(qml, "import QtQuick.Controls").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "Item {{").unwrap();
    writeln!(qml, "    id: root").unwrap();
    writeln!(qml, "    focus: true").unwrap();
    writeln!(qml).unwrap();

    // Properties and signals expected by KScreenLocker.
    writeln!(qml, "    property bool debug: false").unwrap();
    writeln!(qml, "    property string notification").unwrap();
    writeln!(qml, "    signal clearPassword()").unwrap();
    writeln!(qml, "    signal notificationRepeated()").unwrap();
    writeln!(qml, "    property bool viewVisible: false").unwrap();
    writeln!(qml).unwrap();

    // Screenshaver widget definition.
    writeln!(
        qml,
        "    readonly property int childCount: {}",
        CHILD_COUNT
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property int haloRingCount: {}",
        HALO_RING_COUNT
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property real parentRadius: {:.8}",
        config.parent_radius
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property real childRadius: {:.8}",
        config.child_radius
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property real backgroundRadius: {:.8}",
        config.background_radius
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property color childInactiveColor: {}",
        child_inactive_color
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property color childActiveColor: {}",
        child_active_color
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property color childErrorColor: {}",
        child_error_color
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property color backgroundColor: {}",
        background_color
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property color haloColor: {}",
        halo_color
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property real haloStrength: {:.8}",
        config.halo_strength
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property bool randomizeChildDisplay: {}",
        if config.randomize_child_display {
            "true"
        } else {
            "false"
        }
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property int childActiveFadeTime: {}",
        fade_ms
    )
    .unwrap();

    writeln!(
        qml,
        "    readonly property int authenticationFailureDuration: {}",
        failure_ms
    )
    .unwrap();

    writeln!(qml).unwrap();

    // Runtime presentation state. Password contents remain in passwordInput
    // inside the KScreenLocker greeter process.
    writeln!(qml, "    property int nextSequentialChild: 0").unwrap();
    writeln!(qml, "    property int lastRandomChild: -1").unwrap();
    writeln!(qml, "    property int activeChild: -1").unwrap();
    writeln!(qml, "    property int activeKey: -1").unwrap();
    writeln!(qml, "    property bool authenticationFailed: false").unwrap();
    writeln!(qml, "    property bool widgetVisible: true").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    implicitWidth: 800").unwrap();
    writeln!(qml, "    implicitHeight: 600").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    function chooseChild() {{").unwrap();
    writeln!(qml, "        if (!randomizeChildDisplay) {{").unwrap();
    writeln!(qml, "            const chosen = nextSequentialChild").unwrap();
    writeln!(
        qml,
        "            nextSequentialChild = (nextSequentialChild + 1) % childCount"
    )
    .unwrap();
    writeln!(qml, "            return chosen").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(
        qml,
        "        const candidateCount = lastRandomChild >= 0 ? childCount - 1 : childCount"
    )
    .unwrap();
    writeln!(
        qml,
        "        let chosen = Math.floor(Math.random() * candidateCount)"
    )
    .unwrap();
    writeln!(
        qml,
        "        if (lastRandomChild >= 0 && chosen >= lastRandomChild) {{"
    )
    .unwrap();
    writeln!(qml, "            chosen += 1").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "        lastRandomChild = chosen").unwrap();
    writeln!(qml, "        return chosen").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function resetTransientHighlights() {{"
    )
    .unwrap();
    writeln!(qml, "        activeChild = -1").unwrap();
    writeln!(qml, "        activeKey = -1").unwrap();
    writeln!(
        qml,
        "        for (let i = 0; i < childRepeater.count; ++i) {{"
    )
    .unwrap();
    writeln!(qml, "            const child = childRepeater.itemAt(i)").unwrap();
    writeln!(qml, "            if (child) {{").unwrap();
    writeln!(qml, "                child.stopFade()").unwrap();
    writeln!(qml, "                child.highlightAmount = 0.0").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function acceptPasswordText(text, key) {{"
    )
    .unwrap();
    writeln!(
        qml,
        "        if (!text || text.length === 0 || authenticationFailed) {{"
    )
    .unwrap();
    writeln!(qml, "            return").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(qml, "        passwordInput.text += text").unwrap();
    writeln!(qml, "        activeChild = chooseChild()").unwrap();
    writeln!(qml, "        activeKey = key").unwrap();
    writeln!(qml).unwrap();
    writeln!(
        qml,
        "        const child = childRepeater.itemAt(activeChild)"
    )
    .unwrap();
    writeln!(qml, "        if (child) {{").unwrap();
    writeln!(qml, "            child.activateImmediately()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function releasePasswordKey(key) {{"
    )
    .unwrap();
    writeln!(
        qml,
        "        if (authenticationFailed || activeChild < 0 || key !== activeKey) {{"
    )
    .unwrap();
    writeln!(qml, "            return").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(
        qml,
        "        const child = childRepeater.itemAt(activeChild)"
    )
    .unwrap();
    writeln!(qml, "        if (child) {{").unwrap();
    writeln!(qml, "            child.beginFade()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(qml, "        activeChild = -1").unwrap();
    writeln!(qml, "        activeKey = -1").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function removePasswordCharacter() {{"
    )
    .unwrap();
    writeln!(
        qml,
        "        if (authenticationFailed || passwordInput.text.length === 0) {{"
    )
    .unwrap();
    writeln!(qml, "            return").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(
        qml,
        "        passwordInput.text = passwordInput.text.slice(0, -1)"
    )
    .unwrap();
    writeln!(qml, "        resetTransientHighlights()").unwrap();
    writeln!(
        qml,
        "        nextSequentialChild = (nextSequentialChild - 1 + childCount) % childCount"
    )
    .unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function clearAuthenticationDisplay() {{"
    )
    .unwrap();
    writeln!(qml, "        passwordInput.text = \"\"").unwrap();
    writeln!(qml, "        resetTransientHighlights()").unwrap();
    writeln!(qml, "        nextSequentialChild = 0").unwrap();
    writeln!(qml, "        lastRandomChild = -1").unwrap();
    writeln!(qml, "        passwordInput.forceActiveFocus()").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "    function beginAuthenticationFailure() {{"
    )
    .unwrap();
    writeln!(qml, "        passwordInput.text = \"\"").unwrap();
    writeln!(qml, "        resetTransientHighlights()").unwrap();
    writeln!(qml, "        nextSequentialChild = 0").unwrap();
    writeln!(qml, "        lastRandomChild = -1").unwrap();
    writeln!(qml, "        authenticationFailed = true").unwrap();
    writeln!(qml, "        failureTimer.restart()").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    Component.onCompleted: {{").unwrap();
    writeln!(qml, "        passwordInput.forceActiveFocus()").unwrap();
    writeln!(qml, "        authenticator.startAuthenticating()").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    Connections {{").unwrap();
    writeln!(qml, "        target: authenticator").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        function onFailed(kind) {{").unwrap();
    writeln!(qml, "            if (kind !== 0) {{").unwrap();
    writeln!(qml, "                return").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml, "            root.beginAuthenticationFailure()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        function onSucceeded() {{").unwrap();
    writeln!(qml, "            Qt.quit()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "        function onPromptForSecretChanged() {{"
    )
    .unwrap();
    writeln!(qml, "            passwordInput.forceActiveFocus()").unwrap();
    writeln!(qml, "        }}").unwrap();

    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    Timer {{").unwrap();
    writeln!(qml, "        id: failureTimer").unwrap();
    writeln!(
        qml,
        "        interval: root.authenticationFailureDuration"
    )
    .unwrap();
    writeln!(qml, "        repeat: false").unwrap();
    writeln!(qml, "        onTriggered: {{").unwrap();
    writeln!(qml, "            root.authenticationFailed = false").unwrap();
    writeln!(qml, "            passwordInput.forceActiveFocus()").unwrap();
    writeln!(qml, "            authenticator.startAuthenticating()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    Rectangle {{").unwrap();
    writeln!(qml, "        anchors.fill: parent").unwrap();
    writeln!(qml, "        color: \"black\"").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    Item {{").unwrap();
    writeln!(qml, "        id: widgetField").unwrap();
    writeln!(
        qml,
        "        readonly property real haloDistance: root.backgroundRadius * Math.max(0.0, Math.min(root.haloStrength, 1.0))"
    )
    .unwrap();
    writeln!(
        qml,
        "        readonly property real haloOuterRadius: root.backgroundRadius + haloDistance"
    )
    .unwrap();
    writeln!(
        qml,
        "        width: (haloOuterRadius + root.childRadius) * 2.0"
    )
    .unwrap();
    writeln!(qml, "        height: width").unwrap();
    writeln!(qml, "        anchors.centerIn: parent").unwrap();
    writeln!(qml, "        visible: root.widgetVisible").unwrap();
    writeln!(qml).unwrap();

    // Approximate the OpenGL halo's radial alpha falloff with a stack of
    // thin concentric rings. The halo begins at BackgroundRadius and fades
    // smoothly toward BackgroundRadius * (1 + HaloStrength).
    writeln!(qml, "        Repeater {{").unwrap();
    writeln!(qml, "            model: root.haloRingCount").unwrap();
    writeln!(qml).unwrap();
    writeln!(qml, "            Rectangle {{").unwrap();
    writeln!(qml, "                required property int index").unwrap();
    writeln!(
        qml,
        "                readonly property real t: root.haloRingCount <= 1 ? 0.0 : index / (root.haloRingCount - 1)"
    )
    .unwrap();
    writeln!(
        qml,
        "                readonly property real ringRadius: root.backgroundRadius + (widgetField.haloDistance * t)"
    )
    .unwrap();
    writeln!(
        qml,
        "                readonly property real ringStep: widgetField.haloDistance / Math.max(root.haloRingCount, 1)"
    )
    .unwrap();
    writeln!(qml, "                width: ringRadius * 2.0").unwrap();
    writeln!(qml, "                height: width").unwrap();
    writeln!(qml, "                radius: width / 2.0").unwrap();
    writeln!(qml, "                anchors.centerIn: parent").unwrap();
    writeln!(qml, "                color: \"transparent\"").unwrap();
    writeln!(
        qml,
        "                border.width: Math.max(1.0, ringStep + 0.5)"
    )
    .unwrap();
    writeln!(
        qml,
        "                border.color: Qt.rgba(root.haloColor.r, root.haloColor.g, root.haloColor.b, root.haloColor.a * (1.0 - t) * (1.0 - t))"
    )
    .unwrap();
    writeln!(qml, "                visible: widgetField.haloDistance > 0.0").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        Rectangle {{").unwrap();
    writeln!(qml, "            id: backgroundCircle").unwrap();
    writeln!(qml, "            width: root.backgroundRadius * 2.0").unwrap();
    writeln!(qml, "            height: width").unwrap();
    writeln!(qml, "            radius: width / 2.0").unwrap();
    writeln!(qml, "            anchors.centerIn: parent").unwrap();
    writeln!(qml, "            color: root.backgroundColor").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        Repeater {{").unwrap();
    writeln!(qml, "            id: childRepeater").unwrap();
    writeln!(qml, "            model: root.childCount").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "            Rectangle {{").unwrap();
    writeln!(qml, "                id: childCircle").unwrap();
    writeln!(qml, "                required property int index").unwrap();
    writeln!(
        qml,
        "                readonly property real angle: (-Math.PI / 2.0) + (index * ((Math.PI * 2.0) / root.childCount))"
    )
    .unwrap();
    writeln!(qml, "                property real highlightAmount: 0.0").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "                function activateImmediately() {{"
    )
    .unwrap();
    writeln!(qml, "                    fadeAnimation.stop()").unwrap();
    writeln!(qml, "                    highlightAmount = 1.0").unwrap();
    writeln!(qml, "                }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "                function beginFade() {{").unwrap();
    writeln!(qml, "                    fadeAnimation.stop()").unwrap();
    writeln!(qml, "                    fadeAnimation.from = highlightAmount").unwrap();
    writeln!(qml, "                    fadeAnimation.to = 0.0").unwrap();
    writeln!(qml, "                    fadeAnimation.start()").unwrap();
    writeln!(qml, "                }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "                function stopFade() {{").unwrap();
    writeln!(qml, "                    fadeAnimation.stop()").unwrap();
    writeln!(qml, "                }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "                width: root.childRadius * 2.0").unwrap();
    writeln!(qml, "                height: width").unwrap();
    writeln!(qml, "                radius: width / 2.0").unwrap();

    writeln!(
        qml,
        "                x: (widgetField.width / 2.0) + (Math.cos(angle) * root.parentRadius) - (width / 2.0)"
    )
    .unwrap();

    writeln!(
        qml,
        "                y: (widgetField.height / 2.0) + (Math.sin(angle) * root.parentRadius) - (height / 2.0)"
    )
    .unwrap();

    writeln!(
        qml,
        "                color: root.authenticationFailed ? root.childErrorColor : Qt.rgba("
    )
    .unwrap();

    writeln!(
        qml,
        "                    root.childInactiveColor.r + ((root.childActiveColor.r - root.childInactiveColor.r) * highlightAmount),"
    )
    .unwrap();

    writeln!(
        qml,
        "                    root.childInactiveColor.g + ((root.childActiveColor.g - root.childInactiveColor.g) * highlightAmount),"
    )
    .unwrap();

    writeln!(
        qml,
        "                    root.childInactiveColor.b + ((root.childActiveColor.b - root.childInactiveColor.b) * highlightAmount),"
    )
    .unwrap();

    writeln!(
        qml,
        "                    root.childInactiveColor.a + ((root.childActiveColor.a - root.childInactiveColor.a) * highlightAmount)"
    )
    .unwrap();

    writeln!(qml, "                )").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "                NumberAnimation {{").unwrap();
    writeln!(qml, "                    id: fadeAnimation").unwrap();
    writeln!(qml, "                    target: childCircle").unwrap();
    writeln!(qml, "                    property: \"highlightAmount\"").unwrap();
    writeln!(
        qml,
        "                    duration: root.childActiveFadeTime"
    )
    .unwrap();
    writeln!(qml, "                    easing.type: Easing.Linear").unwrap();
    writeln!(qml, "                }}").unwrap();

    writeln!(qml, "            }}").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    // Hidden text input keeps password material inside KScreenLocker's process.
    writeln!(qml, "    TextInput {{").unwrap();
    writeln!(qml, "        id: passwordInput").unwrap();
    writeln!(qml, "        width: 1").unwrap();
    writeln!(qml, "        height: 1").unwrap();
    writeln!(qml, "        opacity: 0").unwrap();
    writeln!(qml, "        echoMode: TextInput.Password").unwrap();
    writeln!(qml, "        focus: true").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        Keys.onPressed: event => {{").unwrap();
    writeln!(
        qml,
        "            if (event.key === Qt.Key_Escape) {{"
    )
    .unwrap();
    writeln!(qml, "                root.dismissAuthenticationDisplay()").unwrap();
    writeln!(qml, "                event.accepted = true").unwrap();
    writeln!(qml, "                return").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml).unwrap();
    writeln!(qml, "            root.revealAuthenticationDisplay()").unwrap();
    writeln!(qml).unwrap();
    writeln!(
        qml,
        "            if (root.authenticationFailed) {{"
    )
    .unwrap();
    writeln!(qml, "                event.accepted = true").unwrap();
    writeln!(qml, "                return").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "            if (event.key === Qt.Key_Backspace) {{"
    )
    .unwrap();
    writeln!(qml, "                root.removePasswordCharacter()").unwrap();
    writeln!(qml, "                event.accepted = true").unwrap();
    writeln!(qml, "                return").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {{"
    )
    .unwrap();
    writeln!(
        qml,
        "                if (passwordInput.text.length > 0) {{"
    )
    .unwrap();
    writeln!(
        qml,
        "                    authenticator.respond(passwordInput.text)"
    )
    .unwrap();
    writeln!(qml, "                }}").unwrap();
    writeln!(qml, "                event.accepted = true").unwrap();
    writeln!(qml, "                return").unwrap();
    writeln!(qml, "            }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(
        qml,
        "            if (event.text && event.text.length > 0 && event.text.charCodeAt(0) >= 0x20) {{"
    )
    .unwrap();
    writeln!(
        qml,
        "                root.acceptPasswordText(event.text, event.key)"
    )
    .unwrap();
    writeln!(qml, "                event.accepted = true").unwrap();
    writeln!(qml, "            }}").unwrap();

    writeln!(qml, "        }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "        Keys.onReleased: event => {{").unwrap();
    writeln!(
        qml,
        "            root.releasePasswordKey(event.key)"
    )
    .unwrap();
    writeln!(qml, "        }}").unwrap();

    writeln!(qml, "    }}").unwrap();
    writeln!(qml).unwrap();

    writeln!(qml, "    MouseArea {{").unwrap();
    writeln!(qml, "        anchors.fill: parent").unwrap();
    writeln!(qml, "        acceptedButtons: Qt.LeftButton").unwrap();
    writeln!(qml, "        onClicked: {{").unwrap();
    writeln!(qml, "            root.revealAuthenticationDisplay()").unwrap();
    writeln!(qml, "            passwordInput.forceActiveFocus()").unwrap();
    writeln!(qml, "        }}").unwrap();
    writeln!(qml, "    }}").unwrap();

    writeln!(qml, "}}").unwrap();

    qml
}

/// Writes the KDE LockScreen.qml representation to `path`.
///
/// This helper intentionally performs no Plasma installation, configuration
/// changes, or KScreenLocker invocation.
pub fn write_lock_screen_kde(
    config: &LockScreenWidgetConfig,
    path: &Path,
) -> io::Result<()> {
    fs::write(
        path,
        construct_lock_screen_kde(config),
    )
}
