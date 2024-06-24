## Implement Leader Elections
   - init replicas with randomized timeouts (Done)
   - identify missing leaders (time since last append entry > randomized timeout) (Done)
   - start an election - send out a RequestVote RPC to all messages and
     transition my state to candidate (Done)
   - vote / respond to RequestVote (I vote for anyone as long as their log
     is at least as long as mine and I haven't voted for someone in the same term)
     (Done)
   - If I receive an append entry from an equal or higher term than mine, I transition from
     candidate to follower (DONE)
   - If I get a majority of votes, transition to state leader and start sending out append entries
     (Done)

