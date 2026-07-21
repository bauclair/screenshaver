//
// display_message.rs
//
// Generic modal message dialogs for Screenshaver.
//

use sdl2::messagebox::{
    show_simple_message_box,
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