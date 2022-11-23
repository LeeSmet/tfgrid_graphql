use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// An uptime event on the grid.
#[derive(Serialize, Deserialize)]
pub struct UptimeEvent {
    #[serde(deserialize_with = "de_timestamp")]
    timestamp: i64,
    #[serde(deserialize_with = "de_uptime")]
    uptime: u64,
}

// Helper function to deserialize a timestamp which is returned as string in graphql for some
// reason
fn de_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num
            .as_i64()
            .ok_or_else(|| de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}

// Helper function to deserialize an uptime which is returned as string in graphql for some
// reason
fn de_uptime<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(s) => s.parse().map_err(de::Error::custom)?,
        Value::Number(num) => num
            .as_u64()
            .ok_or_else(|| de::Error::custom("Invalid number"))?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}
