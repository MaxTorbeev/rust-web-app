use serde::{Deserialize, Serialize};
use crate::UserIdentity;

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub user: UserIdentity
}

impl Session {
    pub fn new(user: UserIdentity) -> Self {
        Self { user }
    }
}

pub enum SessionError {

}
