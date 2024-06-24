use serde::{Deserialize, Serialize};

use crate::replica::LogEntry;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ServerMessage {
    #[serde(flatten)]
    pub body: Body,
    #[serde(flatten)]
    pub options: ServerOptions,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ClientMessage {
    #[serde(flatten)]
    pub body: Body,
    #[serde(flatten)]
    pub options: ClientOptions,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Body {
    pub src: String,
    pub dst: String,
    pub leader: String,
    #[serde(rename = "MID")]
    pub mid: String,
}

// ClientOptions is the set of messages that a replica can send to the client
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ClientOptions {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "ok")]
    WriteOK,
    #[serde(rename = "ok")]
    ReadOk { value: String },
    #[serde(rename = "redirect")]
    Redirect,
}

// ServerOptions is the collection of messages that a replica should receive and
// be expected to handle.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ServerOptions {
    #[serde(rename = "put")]
    Put { key: String, value: String },
    #[serde(rename = "get")]
    Get { key: String },
    // last_log_index is the length of the log
    // last_log_term is the highest term that a replica has in it's log
    #[serde(rename = "request_vote")]
    RequestVote {
        term: u16,
        last_log_index: u16,
        last_log_term: u16,
    },
    #[serde(rename = "vote")]
    Vote { term: u16 },

    #[serde(rename = "append_entry")]
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
    #[serde(rename = "append_entry_result")]
    AppendEntryResult {
        // currentTerm, for leader to update itself
        term: u16,
        // true if follower contained entry matching prevLogIndex and prevLogTerm
        success: bool,
    },
}

// This test only runs on Unix machines - UnixSeqPacket won't compile
// on machines w/o SCTP support
#[cfg(test)]
mod tests {

    use serde_json::json;
    use serde_json::Result;

    use crate::messages::ServerMessage;
    use crate::messages::ServerOptions;

    #[test]
    fn test_ser_de() -> Result<()> {
        let put_msg = json!({
            "src": "0001",
            "dst": "0002",
            "leader": "FFFF",
            "MID": "a message",
            "type": "put",
            "key": "a key",
            "value": "a value",
        });

        let get_msg = json!({
            "src": "0001",
            "dst": "0002",
            "leader": "FFFF",
            "MID": "a message",
            "type": "get",
            "key": "a key",
        });

        let deserialized_put = ServerMessage {
            body: crate::messages::Body {
                src: "0001".to_string(),
                dst: "0002".to_string(),
                leader: "FFFF".to_string(),
                mid: "a message".to_string(),
            },
            options: ServerOptions::Put {
                key: "a key".to_string(),
                value: "a value".to_string(),
            },
        };

        let de_get = ServerMessage {
            body: crate::messages::Body {
                src: "0001".to_string(),
                dst: "0002".to_string(),
                leader: "FFFF".to_string(),
                mid: "a message".to_string(),
            },
            options: ServerOptions::Get {
                key: "a key".to_string(),
            },
        };

        let put_res = serde_json::from_value(put_msg)?;
        println!("{:?}", put_res);

        let get_res = serde_json::from_value(get_msg)?;

        assert_eq!(deserialized_put, put_res);
        assert_eq!(de_get, get_res);

        Ok(())
    }
}
