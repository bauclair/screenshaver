#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserIdentity {
    pub real_uid: libc::uid_t,
    pub effective_uid: libc::uid_t,
}

impl UserIdentity {
    pub fn is_root(self) -> bool {
        self.real_uid == 0 || self.effective_uid == 0
    }
}

pub fn current_user_identity() -> UserIdentity {
    UserIdentity {
        real_uid: unsafe { libc::getuid() },
        effective_uid: unsafe { libc::geteuid() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_real_uid_root() {
        let identity = UserIdentity {
            real_uid: 0,
            effective_uid: 1000,
        };

        assert!(identity.is_root());
    }

    #[test]
    fn detects_effective_uid_root() {
        let identity = UserIdentity {
            real_uid: 1000,
            effective_uid: 0,
        };

        assert!(identity.is_root());
    }

    #[test]
    fn accepts_normal_user() {
        let identity = UserIdentity {
            real_uid: 1000,
            effective_uid: 1000,
        };

        assert!(!identity.is_root());
    }
}