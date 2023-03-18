//! Compile time options.

/// Default time in seconds to keep an item in the clipboard. Should be a low
/// value for security.
pub const DEFAULT_CLIP_TIME: u64 = 10;

/// The default item to view in a group (usually the password). Always an
/// exact match.
pub const DEFAULT_ITEM: &str = "password";

/// Name of the default pass file containing encrypted data. Must be a valid
/// file name.
pub const DEFAULT_PASS_FILE_NAME: &str = "data.pass";

// TODO: necessary ?
/// The maximum number of times to prompt for the password if entered
/// incorrectly.
pub const PASSWORD_ATTEMPTS: u32 = 3;
