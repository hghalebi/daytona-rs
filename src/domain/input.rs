use super::{ShellScript, ValidationError};
use std::fmt;

/// Script input at the public boundary. Strings are validated before execution.
///
/// Owned strings and validated scripts move into this adapter. Borrowed text is
/// copied once into owned input. Debug never displays the script.
pub struct ScriptInput(Input);
enum Input {
    Text(String),
    Validated(ShellScript),
}
impl From<String> for ScriptInput {
    fn from(value: String) -> Self {
        Self(Input::Text(value))
    }
}
impl From<&str> for ScriptInput {
    fn from(value: &str) -> Self {
        Self::from(String::from(value))
    }
}
impl From<ShellScript> for ScriptInput {
    fn from(value: ShellScript) -> Self {
        Self(Input::Validated(value))
    }
}
impl fmt::Debug for ScriptInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ScriptInput([REDACTED])")
    }
}
impl ScriptInput {
    /// Borrows the input text deliberately. It may contain secrets.
    pub fn text(&self) -> &str {
        match &self.0 {
            Input::Text(text) => text,
            Input::Validated(script) => script.as_str(),
        }
    }
    pub(crate) fn check(&self) -> Result<(), ValidationError> {
        ShellScript::validate_text(self.text())
    }
    /// Validates without sending a request, retaining invalid input in the error.
    pub fn validate(self) -> Result<ShellScript, ScriptInputError> {
        if let Err(cause) = ShellScript::validate_text(self.text()) {
            return Err(ScriptInputError { input: self, cause });
        }
        match self.0 {
            Input::Text(text) => Ok(ShellScript(text)),
            Input::Validated(script) => Ok(script),
        }
    }
}
/// Invalid script input retained for correction.
#[derive(Debug, thiserror::Error)]
#[error("script input is invalid: {cause}")]
pub struct ScriptInputError {
    input: ScriptInput,
    #[source]
    cause: ValidationError,
}
impl ScriptInputError {
    /// Borrows the validation failure without exposing the script.
    pub fn cause(&self) -> &ValidationError {
        &self.cause
    }
    /// Returns the original input and its violation without copying either.
    pub fn into_parts(self) -> (ScriptInput, ValidationError) {
        (self.input, self.cause)
    }
}
