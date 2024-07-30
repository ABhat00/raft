use std::collections::BTreeMap;

use super::{
    messages::orchestrator_messages::{
        ClientRequest, InterruptEvent, RaftOrchestratorClientResponse,
    },
    orchestrator::{Orchestrator, RaftState},
    RaftError,
};

pub type Key = u32;
pub type Value = u32;
pub type ReplicaId = u16;
type TimeUnit = u32;

pub struct SimulationDetails {
    // what else exists on a simulation?
    pub num_replicas: u16,
    // in number of messages?
    pub lifetime: TimeUnit,
    pub num_requests: u32,
    pub interrupt_events: Vec<(TimeUnit, InterruptEvent)>,
}

pub struct RaftSimulator {
    pub correct_state: BTreeMap<Key, Value>,
    // These should all come from the raft lib I think
    pub orch: Box<
        dyn for<'a> Orchestrator<
            ClientRequest,
            RaftOrchestratorClientResponse,
            RaftError,
            RaftState<'a>,
            InterruptEvent,
        >,
    >,
    pub sd: SimulationDetails,
}
