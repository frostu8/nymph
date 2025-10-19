//! User database things.

use serde::{Deserialize, Serialize};

/// A single user.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Hash)]
pub struct User {
    /// The unique ID of the user.
    pub id: i32,
    /// The display name of the user.
    pub display_name: String,
}
