pub mod orchestrator_messages {
    use crate::raft::raft_simulator::{Key, ReplicaId, Value};

    pub enum InterruptEvent {
        Kill(ReplicaId),
        EndSimulation,
    }

    // This should probably feel "synchronous" - we shouldn't need to select over this
    pub enum ClientRequest {
        Put(Key, Value),
        Get(Key),
    }

    pub enum RaftOrchestratorClientResponse {
        Fail,
        ReadOk(String),
        WriteOk,
    }
}

pub mod replica_messages {
    use crate::raft::replica::LogEntry;

    use super::orchestrator_messages::ClientRequest;

    // a replica can respond with this to the orchestrator, and the orch will respond
    // to the client
    pub enum RaftReplicaClientResponse {
        Fail,
        ReadOk(String),
        WriteOk,
        // If we send something to the wrong replica, the replica should respond with a redirect to the leader
        Redirect(ClientRequest, String),
    }

    // Replicas can send and receive these to and from each other
    pub enum ReplicaMessage {
        // (term, last_log_index, last_log_term)
        RequestVote {
            term: u16,
            last_log_index: u16,
            last_log_term: u16,
        },
        //term
        Vote(u16),
        // (term, replicaId, )
        AppendEntry {
            term: u16,
            leader_id: String,
            // index of log entry immediately preceding new ones
            prev_log_index: u16,
            // term of prevLogIndex entry
            prev_log_term: u16,
            entries: Vec<LogEntry>,
            // index of the last log entry that the leader has committed
            leader_commit_index: u16,
        },
        // AppendEntry(u16, String, u16, u16, Vec<LogEntry>, u16),
        AppendEntryResult(u16, bool),
    }

    pub enum RaftSend {
        ReplicaMessage(ReplicaMessage),
        RaftReplicaClientResponse(RaftReplicaClientResponse),
    }

    pub enum RaftReceive {
        ReplicaMessage(ReplicaMessage),
        ClientMessage(ClientRequest),
    }
}
