//! Input validation.

use std::fmt::Debug;
use std::ops::RangeBounds;

use crate::app::{AppError, AppErrorKind};

/// A validator.
pub trait Validator<T> {
    /// The name of the value.
    fn name(&self) -> &'static str;

    /// Runs the validation.
    fn validate(self) -> Result<T, AppError>;
}

/// Validator extension functions.
pub trait ValidatorExt<T>
where
    Self: Sized,
{
    /// Checks if a value is in range.
    fn in_range<R>(self, range: R) -> RangeValidator<Self, R>;
}

impl<T, V> ValidatorExt<V> for T
where
    T: Validator<V> + Sized,
{
    fn in_range<R>(self, range: R) -> RangeValidator<Self, R> {
        RangeValidator::new(self, range)
    }
}

/// Represents a value with no constraints.
///
/// This is where all input validation begins. As such, this struct's
/// validation scheme always returns happily.
#[derive(Debug)]
pub struct Value<T> {
    name: &'static str,
    value: T,
}

impl<T> Value<T> {
    /// Creates a new `Validator`.
    pub fn new(name: &'static str, value: T) -> Value<T> {
        Value { name, value }
    }
}

impl<T> Validator<T> for Value<T> {
    fn validate(self) -> Result<T, AppError> {
        Ok(self.value)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// Range validator.
#[derive(Debug)]
pub struct RangeValidator<I, R> {
    inner: I,
    range: R,
}

impl<I, R> RangeValidator<I, R> {
    /// Creates a new `RangeValidator`.
    pub fn new(inner: I, range: R) -> RangeValidator<I, R> {
        RangeValidator { inner, range }
    }
}

impl<T, I, R> Validator<T> for RangeValidator<I, R>
where
    R: RangeBounds<T> + Debug,
    I: Validator<T>,
    T: PartialOrd,
{
    /// Checks if a value is in range.
    ///
    /// Returns `Err` with a descriptive error if it is not in range.
    fn validate(self) -> Result<T, AppError> {
        let name = self.inner.name();
        let value = self.inner.validate()?;

        if self.range.contains(&value) {
            Ok(value)
        } else {
            Err(
                AppError::from(AppErrorKind::FieldOutOfRange(name.to_owned())).with_message(
                    format!(
                        "Field `{}` is out of range; possible values: {:?}",
                        name, self.range
                    ),
                ),
            )
        }
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// Shorthand for [`Value::new`].
pub fn value<T>(name: &'static str, value: T) -> Value<T> {
    Value::new(name, value)
}
