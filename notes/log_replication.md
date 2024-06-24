## Next Milestone - Log Replication: Still need to break down what this is
Here's a rough sketch of how log replication works (written from the leaders POV)

1. Each time I (the leader) get a put request, I send an append entry to all
   of my followers. The append entry contains the index and term of the last
   last log entry that I think we (unique for each leader/replica pair) agree
   on. I track this "match index" for each replica (init to the last index)
   when I get elected

2. If the replica's log agrees with my index and term, it will append everything
   I give it to the log starting at the last index we agree upon OVERWRITING
   ANYTHING THAT MAY EXIST AT THOSE LOCATIONS. It will then send me an AppendEntryResult{TRUE}
   and I can update the "match index" to be the length of my log
   (I know we must match up to the last element I just sent), and the next index
   to be the same

   If it any point there exists an N (N is a log index) such that a majority of the servers
   have match index > N  (and N is greater than the commit_index and the term
   at entry N is the current term), set commit_index to N and commit everything
   up to that point

3. If the replica's log DISAGREES with my index and term, decrement next_index,
   and retry with one less matching element. Once I find the term we agree upon,
   I send the whole set of entries, and overwrite everything the replica currently has
   that doesn't match with us
