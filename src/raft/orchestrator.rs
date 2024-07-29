use super::{raft_simulator::ReplicaId, replica::Replica, RaftError};

pub type RaftState<'a> = Vec<Replica<'a>>;
pub enum MessageDestination {
    Replica(ReplicaId),
    // This is useful for distributing RequestVote RPCs
    EveryoneBut(ReplicaId),
    // Useful for distributing the kill requests
    All,
}

//  This is really just a distributd system cluster
pub trait Orchestrator<Message, ReplicaResponse, ReplicaError, State, InterruptEvent> {
    fn run_orchestrator(num_replicas: u16) -> (Self, Vec<ReplicaId>)
    where
        Self: Sized;

    // aggregate the state of this raft cluster
    fn get_state(self) -> State;
    // forward the message to the appropriate replica(s)
    fn request(
        &self,
        msg: Message,
        msg_destination: MessageDestination,
    ) -> Result<ReplicaResponse, ReplicaError>;

    fn handle_interrupt(
        &self,
        interrupt: InterruptEvent,
        destination: MessageDestination,
    ) -> Result<(), RaftError>;
}
