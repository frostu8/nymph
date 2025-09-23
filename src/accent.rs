//! Accent text provided by the bot in various situations.

use rand::seq::IndexedRandom;

/// All the "not found" lines that the Archivist can say.
pub const NOT_FOUND: &[&str] = &[
    r#""I've never heard of that one before." The Archivist responds with curious eyes. "But it sounds like it might be an interesting addition.""#,
    r#"The Archivist simply responds with a gentle shake of her head at your request."#,
];

/// Gets a random "not found" line.
pub fn roll_not_found() -> &'static str {
    let mut rng = rand::rng();
    NOT_FOUND.choose(&mut rng).expect("at least one line")
}
