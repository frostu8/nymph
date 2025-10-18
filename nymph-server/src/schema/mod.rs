//! SQL schema types and functions.

use nymph_model::card::Visibility as InnerVisibility;

use sqlx::Type;

/// Card visibility enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Type)]
#[repr(i32)]
pub enum Visibility {
    Private = 0,
    Hidden = 1,
    Public = 2,
}

impl From<Visibility> for InnerVisibility {
    fn from(value: Visibility) -> Self {
        match value {
            Visibility::Private => InnerVisibility::Private,
            Visibility::Hidden => InnerVisibility::Hidden,
            Visibility::Public => InnerVisibility::Public,
        }
    }
}

impl From<InnerVisibility> for Visibility {
    fn from(value: InnerVisibility) -> Self {
        match value {
            InnerVisibility::Private => Visibility::Private,
            InnerVisibility::Hidden => Visibility::Hidden,
            InnerVisibility::Public => Visibility::Public,
        }
    }
}
