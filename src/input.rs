use std::collections::HashMap;

pub enum ChordResult {
    Continue,
    Action(String),
    Cancel,
}

pub fn feed_leader(
    acc: &mut String,
    ch: &str,
    leader_keys: &HashMap<String, String>,
) -> ChordResult {
    acc.push_str(ch);

    if let Some(action) = leader_keys.get(acc) {
        return ChordResult::Action(action.clone());
    }

    let possible = leader_keys.keys().any(|k| k.starts_with(acc.as_str()));
    if possible {
        ChordResult::Continue
    } else {
        ChordResult::Cancel
    }
}
