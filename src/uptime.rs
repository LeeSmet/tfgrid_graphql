use crate::compat::{de_i64, de_u64};
use serde::{Deserialize, Serialize};

/// Allowed difference between an advancement in uptime and an advancement in timestamp between 2
/// consecutive events. Currently set to 5 minutes in the minting.
const ALLOWED_UPTIME_DRIFT: i64 = 60 * 5;

/// An uptime event on the grid.
#[derive(Serialize, Deserialize)]
pub struct UptimeEvent {
    #[serde(deserialize_with = "de_i64")]
    timestamp: i64,
    #[serde(deserialize_with = "de_u64")]
    uptime: u64,
}

/// A state change in a node
pub struct NodeStateChange {
    timestamp: i64,
    state: NodeState,
}

impl NodeStateChange {
    /// The timestamp at which the change was detected. Note that this is different than the
    /// timestamp at which the change actually occurred.
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// The new state of the node.
    pub fn state(&self) -> NodeState {
        self.state
    }
}

/// The state of a node.
#[derive(Clone, Copy)]
pub enum NodeState {
    /// Node went offline, enclosed value indicates the timestamp at which point this happened.
    Offline(i64),
    /// Node came (back) online, enclosed value indicates the timestamp at which point this happened.
    Booted(i64),
    /// A reboot is reported which is not possible
    ImpossibleReboot(i64),
    /// Uptime difference compared to timestamp difference is too large. Value is the additional
    /// uptime increase compared to the timestamp increase. A negative value means timestamp
    /// increased more than uptime.
    Drift(i64),
    /// State is unknown, for minting reasons this is presumed down unless a new [`UptimeEvent`]
    /// arrives in time which proves otherwise.
    Unknown(i64),
}

/// Calculate the state changes in a node in a given period based on a series of [`UptimeEvent`]s.
/// It is the callers responsibility to make sure all events in the defined period are given.
///
/// ## Input ordering
///
/// This function assumed the passed [`UptimeEvent`]s are already in sorted in ascending timestamp
/// order. In other words, the oldest event has index 0, the last event (i.e. newest) has the
/// highest index. In case the input might not yet be sorted, a [helper function is provided](calculate_node_state_changes).
///
/// ## Node state at period edge
///
/// Calculating node state at the start of the period is done based on the [`UptimeEvent`] at index
/// 0 (if it exists). Conversely, to known the node state at the end of the period, an
///   [`UptimeEvent`] which occurred __after__ the period ended needs to be provided. It is the
///   caller's responsibility to do this.
pub fn calculate_node_state_changes(
    ues: &[UptimeEvent],
    start: i64,
    end: i64,
) -> Vec<NodeStateChange> {
    let mut state_changes = Vec::new();

    if ues.is_empty() {
        return state_changes;
    }

    // Calculate starting state
    let boot_time = ues[0].timestamp - ues[0].uptime as i64;
    if boot_time > start {
        state_changes.push(NodeStateChange {
            timestamp: ues[0].timestamp,
            state: NodeState::Offline(start),
        });
    }

    state_changes.push(NodeStateChange {
        timestamp: ues[0].timestamp,
        state: NodeState::Booted(boot_time),
    });

    // Calculate state changes
    for window in ues.windows(2) {
        // We expect 1 event to be past the end of the period, and since they are sorted, we know
        // that if the first 1 is past the end, the second one is as well and thus we break here.
        if window[0].timestamp > end {
            break;
        }

        // There are a number of options here. A node is online, so uptime and timestamp both
        // increase with roughly the same value. Or a node is rebooted. In this case, the uptime
        // _MUST_ be less than the difference in timestamp, but can still increase. Or the node is
        // telling us garbage, which must also be caught.
        let ts_delta = window[1].timestamp - window[0].timestamp;

        if (window[1].uptime as i64) < ts_delta {
            // Node went offline
            state_changes.push(NodeStateChange {
                timestamp: window[1].timestamp,
                state: NodeState::Offline(window[0].timestamp),
            });
            // And booted again
            state_changes.push(NodeStateChange {
                timestamp: window[1].timestamp,
                state: NodeState::Booted(window[1].timestamp - window[1].uptime as i64),
            });
            continue;
        }
        // Uptime of second event is bigger than timestamp delta, which can only happen if the node
        // is powered on the whole time, or lying. In the first case, timestamp and uptime need to
        // increase roughly similar.
        // If uptime of the second event is lower than the uptime of the first event, this
        // indicates a reboot, however we already made sure the uptime is bigger than the
        // difference in timestap, meaning the reboot would have happened before the previous
        // report, which is invalid.
        if window[1].uptime < window[0].uptime {
            state_changes.push(NodeStateChange {
                timestamp: window[1].timestamp,
                state: NodeState::ImpossibleReboot(window[1].timestamp - window[1].uptime as i64),
            });
            continue;
        }
        let uptime_delta = window[1].uptime as i64 - window[0].uptime as i64;
        // Check to make sure uptime is not drifting.
        if uptime_delta < ts_delta - ALLOWED_UPTIME_DRIFT
            || uptime_delta > ts_delta + ALLOWED_UPTIME_DRIFT
        {
            state_changes.push(NodeStateChange {
                timestamp: window[1].timestamp,
                state: NodeState::Drift(uptime_delta - ts_delta),
            });
            continue;
        }
        // Regular point, nothing to do. Notice that a node which is offline can't report uptime,
        // therefore the last item in the state change list is always either a boot or a conflict.
    }

    // Check if state at end of period is covered.
    let last_datapoint_timestamp = ues[ues.len() - 1].timestamp;
    if last_datapoint_timestamp < end {
        state_changes.push(NodeStateChange {
            timestamp: end,
            state: NodeState::Unknown(last_datapoint_timestamp),
        });
    }

    state_changes
}

/// Sorts a series of [`UptimeEvent`] in ascending timestamp order.
pub fn sort_uptime_events(ue: &mut [UptimeEvent]) {
    ue.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
}
