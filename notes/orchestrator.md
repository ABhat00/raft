 ##  Timer + Orchestrator + Some refactoring
 
ok here's the plan. We're going to
 1. move the main event loop into the replica impl
 2. set this up to use channels to handle concurrent messages
     - There's going to be a timer channel, and two message channels, for send and receive
     - we select b/w them in the main event loop
     - every time we get a heartbeat, we should reset the timer by creating a new timer channel
 3. We're going to have a separate function for each state - leader, follower, candidate (and introduce three states). It's currently unclear how we're going to handle the follower state, but that's a later problem
 5. Then we're going to abstract away the actual raft component into a library and create an orchestrator layer (RaftOrchestrator)
   - The orchestrator will start some number of replicas (each on its own thread) and send messages to them in the same way the current python simulator
    - This orchestrator should be able to kill replicas while the process is running, and add new replicas
   - This will allow me to unit test things a bit more thoroughly (i guess this means that I need to be able to pass in election timeouts too)
   - This will also allow me to set up the raft simulation to terminate cleanly. It doesn't really do that right now
   - This impl should have two responsiblities:
        1. Manage the simulation (SimulationManager)
            this means it needs to generate messages to send to the leader, and send them on the right interval
             (or any other replica), validate that the responses are correct, and log statistics
            -  two impls are necessary here -
                    I want this to be able to hook into the shitty fd setup from the khoury proj
                    I also want to be able to pass in some SimulationDetails struct that i load from a test file to be able to run this on the web
                    ^ I want to do this part first - dealing with the khoury bullshit seems annoying
        2. Distribute messages between replicas (MessageOrchestrator). These can happen on separate threads in the orchestrator, but I need both to write to the correct replicas.

            So the main event loop is going to look something like
            ``` rust 
            type ReplicaInfo = (Replica, Sender, Receiver)
            struct Orchestrator {
                mut leader: String (Does the cluster orchestrator care about who the leader is?)
                correctState Map<Key, Value>
                r Vec<ReplicaInfo> 
                sm SimulationManager
                mo MessageOrchestrator
                em StateLogger (every time there's an event, i should append
                 to a log in the event manager ordered by some timestamp
                  so that I can render the state of each replica on the web)
            }
            
            impl Orchestrator {
                fn new(num_replicas: int8 , simulation_details: SimulationDetails) -> Orchestrator {
                    let replicaDetails = ... create all the replicas w/ the correct replica id and correct colleageu ids ...

                    Orchestrator {
                        // This isn't an entirely accurate representation of how this should work. The top level orchestrator should send relevant requests to the client
                         and pass everything else between replicas
                        // Ok so I'll have a list of clients that's responsible for managing client state, and validating that the values are correct
                        // This isn't even necessary
                        // And a main event runner that handles initiating messages + partitions + kills
                        //

                        sm: SimulationManager::new(SimulationDetails, replicaDetails.m)
                        // I think the right pattern is to have a client
                        // and this is our simulation manager
                        mo: MessageOrchestrator
                    }
                }

                fn new_replica(replica id: &str , colleague_ids  ) -> ReplicaInfo  {
                 let (tx, rx) = bounded(n);
                 let replica = Replica::new(replica_id, colleague_id, tx, rx)

                 (replica, tx, rx)
                }
            }
            fn run_orchestrator(num_replicas: int8, ) {

            }
            ```

        I think I should flip this - 
      A simulation should be the top level struct and have a `SimulationDetails` struct, as well as be responsible for tracking the simulation Data. this should also have a instance of a RaftOrchestrator. The raft orchestrator should be responsible for forwarding the request created by the simulation to each replica, as well as orchestrating messages between replicas

      ```rust 
        struct RaftSimulator {
          correct_state: Map<Key, Value>   
          orch: RaftOrchestrator
          sd: SimulationDetails
          stats: SimulationStatistics
        }

        trait Simulator {
            pub fn run() -> Result<(), Error>
        }

		type ReplicaInfo = (Replica, Sender, Receiver)

        struct RaftOrchestrator {
            rs Vec<ReplicaInfo>
            em StateLogger
        }

        trait Orchestrator {
            fn run_orchestrator(num_replicas: i32) -> (RaftOrchestrator, Vec<ReplicaId>);
            fn get_state(self) -> ClusterState;
            fn send(msg) -> Result<ReplicaResponse, Error>;
            fn kill_replica(replica_id: ReplicaId) -> Result<(), Error>
        }
      ```

      The orchestrator is responsible for starting each replica on a different thread, managing communication between replicas, and replying to the client (the simulator)