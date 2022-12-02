use crate::{
    bill_report::ContractBillReport,
    contract::{ContractState, NodeContract},
    uptime::UptimeEvent,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Amount of items to fetch when iterating on graphql.
const PAGE_SIZE: usize = 200;

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
const CONTRACT_BILL_REPORT_QUERY: &str = r#"
query get_contract_bill_reports($start: BigInt, $end: BigInt) {
  contractBillReports(where: {timestamp_gte: $start, timestamp_lte: $end}, orderBy: timestamp_ASC) {
    amountBilled
    contractID
    timestamp
    discountReceived
  }
}
"#;
const CONTRACTS_QUERY: &str = r#"
query contracts($nodes: [Int!], $states: [ContractState!], $twins: [Int!], $offset: Int) {
  nodeContracts(where: {nodeID_in: $nodes, state_in: $states, twinID_in: $twins}, orderBy: contractID_ASC, limit: 200, offset: $offset) {
    contractID
    createdAt
    deploymentData
    deploymentHash
    gridVersion
    nodeID
    numberOfPublicIPs
    resourcesUsed {
      cru
      hru
      mru
      sru
    }
    solutionProviderID
    state
    twinID
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

#[derive(Serialize)]
struct UptimeVariables {
    node_id: u32,
    start: i64,
    end: i64,
}

#[derive(Serialize)]
struct ContractBillReportVariables {
    start: i64,
    end: i64,
}

#[derive(Serialize)]
struct ContractsVariables<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    nodes: Option<&'a [u32]>,
    states: &'a [ContractState],
    #[serde(skip_serializing_if = "Option::is_none")]
    twins: Option<&'a [u32]>,
    offset: usize,
}

#[derive(Deserialize)]
struct UptimeEventResponse {
    #[serde(rename = "uptimeEvents")]
    uptime_events: Vec<UptimeEvent>,
}

#[derive(Deserialize)]
struct ContractBillEventResponse {
    #[serde(rename = "contractBillReports")]
    contract_bill_reports: Vec<ContractBillReport>,
}

#[derive(Deserialize)]
struct ContractsResponse {
    #[serde(rename = "nodeContracts")]
    node_contracts: Vec<NodeContract>,
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

    // TODO: make these methods a single generic with a trait + associated type on
    // request/response

    /// Fetch the uptime events for the given node in the given time range. The returned values are
    /// requested to be sorted in ascending timestamp order from the server.
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

    /// Fetch all contract bill reports in the given time range.
    pub fn contract_bill_reports(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<ContractBillReport>, Box<dyn std::error::Error>> {
        Ok(self
            .client
            .post(&self.endpoint)
            .json(&GraphQLRequest {
                operation_name: "get_contract_bill_reports",
                query: CONTRACT_BILL_REPORT_QUERY,
                variables: Some(&ContractBillReportVariables { start, end }),
            })
            .send()?
            .json::<GraphQLResponse<ContractBillEventResponse>>()?
            .data
            .contract_bill_reports)
    }

    /// Fetch all contracts in the given states from the given nodes.
    pub fn contracts(
        &self,
        nodes: Option<&[u32]>,
        states: &[ContractState],
        twins: Option<&[u32]>,
    ) -> Result<Vec<NodeContract>, Box<dyn std::error::Error>> {
        let mut contracts = Vec::new();
        let mut offset = 0;
        loop {
            let mut new_contracts = self
                .client
                .post(&self.endpoint)
                .json(&GraphQLRequest {
                    operation_name: "contracts",
                    query: CONTRACTS_QUERY,
                    variables: Some(&ContractsVariables {
                        nodes,
                        states,
                        twins,
                        offset,
                    }),
                })
                .send()?
                .json::<GraphQLResponse<ContractsResponse>>()?
                .data
                .node_contracts;
            let found_objects = new_contracts.len();
            offset += found_objects;
            contracts.append(&mut new_contracts);
            if found_objects != PAGE_SIZE {
                break;
            }
        }
        Ok(contracts)
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

    #[test]
    fn fetch_contract_bill_reports() {
        let cl =
            Client::new("https://graph.grid.tf/graphql".to_string()).expect("Can create a client");

        let ues = cl
            .contract_bill_reports(1663850262, 1663857474)
            .expect("Can fetch contract bill events from mainnet");

        assert_eq!(ues.len(), 223);
    }
}
