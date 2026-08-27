use crate::UserIdentity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Session {
  pub user: UserIdentity,
}
