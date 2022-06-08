use std::char::DecodeUtf16;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Send {
    #[serde(flatten)]
    pub body: Body,
    #[serde(flatten)]
    pub options: SendOptions,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Recv {
    #[serde(flatten)]
    pub body: Body,
    #[serde(flatten)]
    pub options: RecvOptions,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Body {
    pub src: String,
    pub dst: String,
    pub leader: String,
    #[serde(rename = "MID")]
    pub mid: String,
}

// RecvOptions denotes the collection of messages that a replica should receive and
// be expected to handle.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum RecvOptions {
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
}

// SendOptions is the collection of messages that a replica can send
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum SendOptions {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "ok")]
    WriteOK,
    #[serde(rename = "ok")]
    ReadOk { value: String },
    #[serde(rename = "redirect")]
    Redirect,
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
}

// This test only runs on Unix machines - UnixSeqPacket won't compile
// on machines w/o SCTP support
#[cfg(test)]
mod tests {

    use serde_json::json;

    use crate::messages;
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

        let deserialized_put = Recvs {
            src: "0001".to_string(),
            dst: "0002".to_string(),
            leader: "FFFF".to_string(),
            msg_id: "a message".to_string(),
            msg_type: Recv::Put {
                key: "a key".to_string(),
                value: "a value".to_string(),
            },
        };

        let de_get = Body {
            src: "0001".to_string(),
            dst: "0002".to_string(),
            leader: "FFFF".to_string(),
            msg_id: "a message".to_string(),
            msg_type: Recv::Get {
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
