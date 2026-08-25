// PAM authentication boundary for Screenshaver.
//
// This module verifies credentials only.  It has no access to the Wayland
// session-lock object and therefore cannot unlock a session.

const PAM_SERVICE: &str = "login";


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationResult {
    Success,
    Rejected,
    Error(String),
}


pub fn authenticate(
    username: &str,
    password: &str,
) -> AuthenticationResult {
    let mut client =
        match pam::Client::with_password(
            PAM_SERVICE
        ) {
            Ok(client) => {
                client
            }

            Err(error) => {
                return AuthenticationResult::Error(
                    format!(
                        "Unable to initialize PAM service '{}': {}",
                        PAM_SERVICE,
                        error,
                    )
                );
            }
        };


    client
        .conversation_mut()
        .set_credentials(
            username,
            password,
        );


    match client.authenticate() {
        Ok(()) => {
            AuthenticationResult::Success
        }

        Err(error) => {
            use pam::PamReturnCode;


            match error.0 {
                PamReturnCode::Auth_Err
                | PamReturnCode::User_Unknown
                | PamReturnCode::MaxTries
                | PamReturnCode::Cred_Insufficient => {
                    AuthenticationResult::Rejected
                }

                _ => {
                    AuthenticationResult::Error(
                        format!(
                            "PAM authentication error: {}",
                            error,
                        )
                    )
                }
            }
        }
    }
}
