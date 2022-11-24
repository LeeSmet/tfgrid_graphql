use crate::uptime::UptimeEvent;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const USER_AGENT: &str = "tfgrid_graphql_client";
const MAINNET_URL: &str = "https://graph.grid.tf/graphql";
const UPTIME_EVENT_QUERY: &str = r#"
query get_uptime_events($node_id: Int, $start: BigInt, $end: BigInt) {
    uptimeEvents(where: {nodeID_eq: $node_id, timestamp_gte: $start, timestamp_lte: $end}, orderBy: timestamp_ASC) {
        timestamp
        uptime
    }
}
"#;

/// A client to connect to a Threefold Grid GraphQL instance.
pub struct Client {
    endpoint: String,
    client: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct GraphQLRequest<'a, T: Serialize> {
    operation_name: &'a str,
    query: &'a str,
    // TODO
    variables: Option<T>,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

#[derive(Serialize, Deserialize)]
struct UptimeVariables {
    node_id: u32,
    start: i64,
    end: i64,
}

#[derive(Deserialize)]
struct UptimeEventResponse {
    #[serde(rename = "uptimeEvents")]
    uptime_events: Vec<UptimeEvent>,
}

impl Client {
    /// Creates a new Client which will connect to the given endpoint. No validation is done on the
    /// url at this stage.
    pub fn new(endpoint: String) -> Result<Client, Box<dyn std::error::Error>> {
        Ok(Client {
            endpoint,
            client: reqwest::blocking::ClientBuilder::new()
                .gzip(true)
                .connect_timeout(Duration::from_secs(5))
                .user_agent(USER_AGENT)
                .build()?,
        })
    }

    /// Creates a new client connected to the mainnet graphql instance.
    pub fn mainnet() -> Result<Client, Box<dyn std::error::Error>> {
        Self::new(MAINNET_URL.to_string())
    }

    /// Fetch the uptime events for the given node in the given time range. The returned values are
    /// requested to be sorted in ascneding timetamp order from the server.
    pub fn uptime_events(
        &self,
        node_id: u32,
        start: i64,
        end: i64,
    ) -> Result<Vec<UptimeEvent>, Box<dyn std::error::Error>> {
        Ok(self
            .client
            .post(&self.endpoint)
            .json(&GraphQLRequest {
                operation_name: "get_uptime_events",
                query: UPTIME_EVENT_QUERY,
                variables: Some(&UptimeVariables {
                    node_id,
                    start,
                    end,
                }),
            })
            .send()?
            .json::<GraphQLResponse<UptimeEventResponse>>()?
            .data
            .uptime_events)
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn fetch_uptime_events() {
        let cl =
            Client::new("https://graph.grid.tf/graphql".to_string()).expect("Can create a client");

        let ues = cl
            .uptime_events(4200, 1663850262, 1663857474)
            .expect("Can fetch uptime events from mainnet");

        assert_eq!(ues.len(), 2);
    }
}
