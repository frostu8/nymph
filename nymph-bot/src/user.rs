//! User interactions.

use std::fmt::{self, Display, Formatter};

use twilight_model::id::{Id, marker::UserMarker};

use crate::commands::InteractionContext;

/// Checks if a user can be `/grant`ed cards.
pub fn validate_grant(
    cx: &InteractionContext,
    user_id: impl Into<Id<UserMarker>>,
) -> Result<(), GrantTargetError> {
    let target_user = cx.cache.user(user_id.into()).expect("cached user");
    let is_current_bot = cx
        .cache
        .current_user()
        .map(|current_user| current_user.id == target_user.id)
        .unwrap_or(false);

    if is_current_bot {
        Err(GrantTargetError::IsCurrentUser)
    } else if target_user.bot {
        Err(GrantTargetError::IsBot)
    } else {
        Ok(())
    }
}

/// An error for grant targets.
#[derive(Clone, Debug)]
pub enum GrantTargetError {
    /// The target is a current user.
    IsCurrentUser,
    /// The target is a bot.
    IsBot,
}

impl Display for GrantTargetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GrantTargetError::IsCurrentUser => f.write_str("user is current bot"),
            GrantTargetError::IsBot => f.write_str("user is a bot"),
        }
    }
}
