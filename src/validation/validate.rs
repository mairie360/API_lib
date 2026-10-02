use super::ValidationError;

/// Implemented by every request view extracted with [`super::ValidatedJson`] or
/// [`super::ValidatedQuery`].
pub trait Validate {
    /// # Errors
    ///
    /// Returns the first field that does not satisfy its constraints.
    fn validate(&self) -> Result<(), ValidationError>;
}
