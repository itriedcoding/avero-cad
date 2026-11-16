//! ER:LC API integration (v2 primary, v1 for the bans list).

pub mod client;

pub use client::{ApiClient, ResponseMeta, ServerQuery};

use crate::models::{split_player, LiveBanView};
use serde_json::Value;

/// Parse `GET /v1/server/bans`.
///
/// The published schema for this endpoint is underspecified (it declares a
/// single `PlayerId` property), and the payload observed in the wild is a list.
/// This parser accepts every shape the documentation implies without inventing
/// ban records: arrays of strings, arrays of objects, id→name maps, and the
/// single-object form.
pub fn parse_bans(value: &Value) -> Vec<LiveBanView> {
    match value {
        Value::Array(items) => items.iter().filter_map(ban_from_value).collect(),
        Value::Object(map) => {
            for key in ["bans", "Bans", "data", "Data"] {
                if let Some(inner) = map.get(key) {
                    return parse_bans(inner);
                }
            }
            if let Some(single) = ban_from_value(value) {
                // A single ban record object.
                if map.keys().any(|k| {
                    matches!(
                        k.as_str(),
                        "PlayerId" | "playerId" | "id" | "Id" | "username" | "Username" | "name"
                    )
                }) && !looks_like_id_map(map)
                {
                    return vec![single];
                }
            }
            map.iter()
                .filter_map(|(id_key, name_value)| {
                    let username = name_value.as_str().map(str::to_string);
                    let user_id = id_key
                        .parse::<u64>()
                        .ok()
                        .or_else(|| username.as_deref().and_then(|name| split_player(name).1));
                    if username.is_none() && user_id.is_none() {
                        return None;
                    }
                    Some(LiveBanView {
                        username: username.map(|n| split_player(&n).0),
                        roblox_user_id: user_id,
                        raw: format!("{id_key}: {}", name_value),
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn looks_like_id_map(map: &serde_json::Map<String, Value>) -> bool {
    !map.is_empty() && map.keys().all(|k| k.parse::<u64>().is_ok())
}

fn ban_from_value(value: &Value) -> Option<LiveBanView> {
    match value {
        Value::String(raw) => {
            let (username, id) = split_player(raw);
            Some(LiveBanView {
                username: Some(username),
                roblox_user_id: id,
                raw: raw.clone(),
            })
        }
        Value::Number(number) => Some(LiveBanView {
            username: None,
            roblox_user_id: number.as_u64(),
            raw: number.to_string(),
        }),
        Value::Object(map) => {
            let name = ["username", "Username", "name", "Name", "Player", "player"]
                .iter()
                .find_map(|k| map.get(*k).and_then(|v| v.as_str()))
                .map(str::to_string);
            let id = [
                "PlayerId", "playerId", "id", "Id", "userId", "UserId", "UserId ",
            ]
            .iter()
            .find_map(|k| {
                map.get(*k).and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                        .or_else(|| v.as_str().and_then(|s| split_player(s).1))
                })
            });
            if name.is_none() && id.is_none() {
                return None;
            }
            let (username, split_id) = name
                .as_deref()
                .map(split_player)
                .unwrap_or((String::new(), None));
            Some(LiveBanView {
                username: if username.is_empty() {
                    None
                } else {
                    Some(username)
                },
                roblox_user_id: id.or(split_id),
                raw: value.to_string(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_array_of_strings() {
        let value = serde_json::json!(["troll:123", "Another:456"]);
        let bans = parse_bans(&value);
        assert_eq!(bans.len(), 2);
        assert_eq!(bans[0].username.as_deref(), Some("troll"));
        assert_eq!(bans[0].roblox_user_id, Some(123));
    }

    #[test]
    fn parses_array_of_objects() {
        let value =
            serde_json::json!([{ "PlayerId": "troll:123" }, { "username": "bob", "id": 9 }]);
        let bans = parse_bans(&value);
        assert_eq!(bans.len(), 2);
        assert_eq!(bans[0].roblox_user_id, Some(123));
        assert_eq!(bans[1].username.as_deref(), Some("bob"));
        assert_eq!(bans[1].roblox_user_id, Some(9));
    }

    #[test]
    fn parses_id_to_name_map() {
        let value = serde_json::json!({ "123": "troll", "456": "bob" });
        let bans = parse_bans(&value);
        assert_eq!(bans.len(), 2);
        assert!(bans.iter().any(|b| b.username.as_deref() == Some("troll")));
    }

    #[test]
    fn parses_wrapper_object() {
        let value = serde_json::json!({ "bans": ["troll:1"] });
        assert_eq!(parse_bans(&value).len(), 1);
    }

    #[test]
    fn empty_or_unexpected_payloads_do_not_panic() {
        assert!(parse_bans(&serde_json::json!(null)).is_empty());
        assert!(parse_bans(&serde_json::json!([])).is_empty());
        assert!(parse_bans(&serde_json::json!({})).is_empty());
    }
}
