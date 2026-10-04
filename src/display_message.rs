//
// display_message.rs
//
// Generic modal message dialogs for Screenshaver.
//

use sdl2::messagebox::{
    show_simple_message_box,
    show_message_box,
    ClickedButton,
    ButtonData,
    MessageBoxButtonFlag,
    MessageBoxFlag,
};


/// Display an informational message.
///
/// Returns after the user dismisses the dialog.
pub fn show_information(
    title: &str,
    message: &str,
) -> Result<(), String> {

    show_message(
        MessageBoxFlag::INFORMATION,
        title,
        message,
    )
}


/// Display a warning message.
///
/// Returns after the user dismisses the dialog.
pub fn show_warning(
    title: &str,
    message: &str,
) -> Result<(), String> {

    show_message(
        MessageBoxFlag::WARNING,
        title,
        message,
    )
}


/// Display an error message.
///
/// Returns after the user dismisses the dialog.
pub fn show_error(
    title: &str,
    message: &str,
) -> Result<(), String> {

    show_message(
        MessageBoxFlag::ERROR,
        title,
        message,
    )
}


/// Internal helper shared by all public dialog functions.
fn show_message(
    flag: MessageBoxFlag,
    title: &str,
    message: &str,
) -> Result<(), String> {

    show_simple_message_box(
        flag,
        title,
        message,
        None,
    )
    .map_err(|error| error.to_string())
}

/// Display a modal warning with Continue and Cancel choices.
/// Closing the dialog is equivalent to Cancel.
pub fn confirm_warning(
    title: &str,
    message: &str,
    continue_label: &str,
    cancel_label: &str,
) -> Result<bool, String> {
    let buttons = [
        ButtonData {
            flags: MessageBoxButtonFlag::RETURNKEY_DEFAULT,
            button_id: 1,
            text: continue_label,
        },
        ButtonData {
            flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
            button_id: 0,
            text: cancel_label,
        },
    ];

    match show_message_box(
        MessageBoxFlag::WARNING,
        &buttons,
        title,
        message,
        None,
        None,
    ).map_err(|error| error.to_string())? {
        ClickedButton::CustomButton(button) => Ok(button.button_id == 1),
        ClickedButton::CloseButton => Ok(false),
    }
}
