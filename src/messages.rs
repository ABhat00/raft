use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize};
use serde_json::json;
use serde_test::{assert_de_tokens, assert_ser_tokens, Token};
use std::io::Result;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Body {
    src: String,
    dst: String,
    leader: String,
    #[serde(rename = "MID")]
    msg_id: String,
    #[serde(flatten)]
    msg_type: Recv,
}

// Recv denotes the collection of messages that a replica should receive and
// be expected to handle. New message
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum Recv {
    #[serde(rename = "put")]
    Put { key: String, value: String },
    #[serde(rename = "get")]
    Get { key: String },
}

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

    let deserialized_put = Body {
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
