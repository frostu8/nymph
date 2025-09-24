//! Accent text provided by the bot in various situations.

use rand::seq::IndexedRandom;

/// All the "not found" lines that the Archivist can say.
pub const NOT_FOUND: &[&str] = &[
    r#""I've never heard of that one before." The Archivist responds with curious eyes. "But it sounds like it might be an interesting addition.""#,
    r#"The Archivist simply responds with a gentle shake of her head at your request."#,
    r#""My superior has dedicated his entire life to the search for absolute knowledge, so I find it unlikely we don't know of that." The Archivist replies with unsure, quaking words."#,
    r#"The Archivist idly folds a paper crane, her attention divided on her task as she balances her wooden chair on two legs. "It probably doesn't exist.""#,
    r#""Maybe you should ask Remy." The Archivist says meekly, her gaze averted. "I've been told 56 is a bit young to be working here...""#,
    r#""I don't know what you could be referring to." The Archivist adjusts a golden ribbon on her crimson hair, her gaze drifting upwards in a weak attempt to visualize herself. "Do you think this looks cute?""#,
    r#"The Archivist shakes her head in an erratic rhythm, her face permanently frozen in a state of shock. "I-I-I don't k-know... I-I'm s-s-sorry! I-I-I was on th-th-thought f-furnace d-d-duty today...""#,
    r#"The Archivist gnaws on a pen. "My boss told me that I can't say we don't know it anymore." She replied, though the pen in her mouth didn't make it any easier to understand. "So we have determined you are delusional, or have had memories implanted.""#,
];

/// Gets a random "not found" line.
pub fn roll_not_found() -> &'static str {
    let mut rng = rand::rng();
    NOT_FOUND.choose(&mut rng).expect("at least one line")
}
