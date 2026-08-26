use zeroize::Zeroizing;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationAction {
    None,
    Dismiss,
    Submit,
}


pub struct LockAuthentication {
    username: String,
    password: Zeroizing<String>,
    status: Option<String>,
    revision: u64,

    widget:
        crate::lock_screen_widget::LockScreenWidget,
}


impl LockAuthentication {
    pub fn new() -> Self {
        let username =
            std::env::var(
                "USER"
            )
            .or_else(
                |_| {
                    std::env::var(
                        "LOGNAME"
                    )
                }
            )
            .unwrap_or_else(
                |_| {
                    "current user"
                        .to_string()
                }
            );


        Self {
            username,

            password:
                Zeroizing::new(
                    String::new()
                ),

            status:
                None,

            revision:
                1,

            widget:
                crate::lock_screen_widget::LockScreenWidget::new(),
        }
    }


    pub fn username(
        &self
    ) -> &str {
        &self.username
    }


    pub fn password_length(
        &self
    ) -> usize {
        self.password
            .chars()
            .count()
    }


    pub fn status(
        &self
    ) -> Option<&str> {
        self.status.as_deref()
    }


    pub fn revision(
        &self
    ) -> u64 {
        self.revision
    }


    pub fn widget(
        &self
    ) -> &crate::lock_screen_widget::LockScreenWidget {
        &self.widget
    }


    pub fn clear(
        &mut self
    ) {
        self.password.clear();

        self.status =
            None;

        self.widget.clear();

        self.bump_revision();
    }


    /// Clear the visual key-down highlight when Wayland reports a physical
    /// keyboard release. No password data is changed.
    pub fn handle_key_release(
        &mut self,
    ) {
        self.widget.key_released();
    }


    pub fn handle_key(
        &mut self,
        keysym: u32,
        utf8: &str,
    ) -> AuthenticationAction {
        use xkbcommon::xkb::keysyms;


        // While the red authentication-failure indication is visible, ignore
        // password input so the next attempt starts only after the widget has
        // returned to its inactive state. Escape remains available to dismiss
        // the authentication UI.
        if self.widget.error_is_active()
            && keysym != keysyms::KEY_Escape
        {
            return AuthenticationAction::None;
        }


        match keysym {
            keysyms::KEY_Escape => {
                self.clear();

                AuthenticationAction::Dismiss
            }


            keysyms::KEY_BackSpace => {
                if self.password.pop().is_some() {
                    self.status =
                        None;

                    self.widget.backspace();

                    self.bump_revision();
                }

                AuthenticationAction::None
            }


            keysyms::KEY_Return
            | keysyms::KEY_KP_Enter => {
                // The caller owns authentication policy. Enter merely requests
                // submission; it never grants unlock authority.
                AuthenticationAction::Submit
            }


            _ => {
                let printable =
                    utf8
                        .chars()
                        .filter(
                            |character| {
                                !character.is_control()
                            }
                        )
                        .collect::<String>();


                if !printable.is_empty() {
                    let remaining =
                        256_usize
                            .saturating_sub(
                                self.password
                                    .chars()
                                    .count()
                            );


                    if remaining > 0 {
                        self.password.extend(
                            printable
                                .chars()
                                .take(
                                    remaining
                                )
                        );

                        self.status =
                            None;

                        // One visual step per accepted physical key press,
                        // independent of the resulting UTF-8 byte length.
                        self.widget.key_pressed();

                        self.bump_revision();
                    }
                }


                AuthenticationAction::None
            }
        }
    }


    pub fn take_password(
        &mut self
    ) -> Zeroizing<String> {
        let password =
            std::mem::replace(
                &mut self.password,
                Zeroizing::new(
                    String::new()
                ),
            );

        self.bump_revision();

        password
    }


    pub fn set_status(
        &mut self,
        status: impl Into<String>,
    ) {
        self.status =
            Some(
                status.into()
            );

        self.bump_revision();
    }


    /// Begin the visual authentication-failure indication.
    ///
    /// This is deliberately separate from `set_status()`: the status remains
    /// available for diagnostics/logging while the lock screen itself remains
    /// completely text-free.
    pub fn authentication_failed(
        &mut self,
    ) {
        self.password.clear();

        self.widget.authentication_failed();

        self.bump_revision();
    }


    fn bump_revision(
        &mut self
    ) {
        self.revision =
            self.revision
                .wrapping_add(
                    1
                );
    }
}


impl Default
    for LockAuthentication
{
    fn default() -> Self {
        Self::new()
    }
}


//
// ------------------------------------------------------------
// Text-free lock-screen authentication panel
// ------------------------------------------------------------
//

pub struct LockAuthenticationPanel {
    widget_renderer:
        crate::lock_screen_widget::LockScreenWidgetRenderer,
}


impl LockAuthenticationPanel {
    pub fn new(
        _authentication: &LockAuthentication,
        _output_width: u32,
        _output_height: u32,
    ) -> Result<Self, String> {
        Ok(
            Self {
                widget_renderer:
                    crate::lock_screen_widget::LockScreenWidgetRenderer::new()?,
            }
        )
    }


    pub fn display(
        &self,
        authentication: &LockAuthentication,
        output_width: u32,
        output_height: u32,
    ) {
        self.widget_renderer
            .display_centered(
                authentication.widget(),
                output_width,
                output_height,
            );
    }
}
