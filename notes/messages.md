### How do i structure messages?

- I have three components of this system:
 -  The simulator (responsible for generating requests, interrupt events and sending those to the orchestrator)
 - The orchestrator, who is responsible for starting the cluster, handling interrupts, sending read or write requests to the cluster, sending messages b/w replicas, and sending the results of a read or write operation to the simualator
 - A replica, who is responsible for handling read and write requests, and sending the required events to other replicas through the orchestrator



I need a shared enum between the orchestraor and the simulator
-  The simulator can send a `Put(key, value)`, a `Get(key)`, a `Kill(rid)`,  or an `End()`. The simulator can receive a `PutResponse(ok?)` or a `GetResponse(value)`
- The orchestrator can receive a `Put(key, value)`, a `Get(key)`, a `Kill(rid)`,  or an `End()` from the simulator. It can then send a `Put(key, value)` , a `Get(key)`  

Here's the structure that comes out of that definition


```rust

pub enum InterruptEvent {
    Kill(ReplicaId),
    EndSimulation(),
}

// This should probably feel "synchronous" - we shouldn't need to select over this
pub enum ClientRequest {
	Put(Key, Value),
	Get(Key),
}

pub RaftOrchestratorClientResponse {
	Fail,
	Ok(String),
	WriteOk
}

// a replica can respond with this to the orchestrator, and the orch will respond
// to the client
pub enum RaftReplicaClientResposne {
	Fail,
	Ok(String),
	WriteOk,
	// If we send something to the wrong replica, the replica should respond with a redirect
	Redirect
}

// Replicas can send and receive these to and from each other
pub enum ReplicaMessage {
	Put(Key, Value)
	Get(Key),
	RequestVote(term, last_log_index, last_log_term)
	Vote(term),
	AppendEntry(term, leader_id, prev_log_index, prev_log_term, entries, leader_commit_index)
	AppendEntryResult(term, success)
}


pub enum RaftSend {
	A(ReplicaMessage)
	B(RaftReplicaClientResponse)
}

pub enum RaftReceive {
	A(ReplicaMessage)
	B(ClientRequest)
}
```

Then the actual replica has a receive channel for `RaftReceive`, and a send channel for `RaftSend`

```rust

struct Replica {
	tx: Sender<(RaftSend, MessageDestination)>
	rx: Receiver<RaftReceive>
}
```
