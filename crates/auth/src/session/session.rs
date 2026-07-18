use serde::{Deserialize, Serialize};
use crate::UserIdentity;

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub user: UserIdentity
}

pub enum SessionError {

}
