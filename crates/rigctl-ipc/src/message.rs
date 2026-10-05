use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::RigCtlError;

pub const PROTOCOL_VERSION: u32 = 1;

pub type RequestId = u64;
pub type JsonObject = Map<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum RequestType {
    #[serde(rename = "request")]
    Request,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ResponseType {
    #[serde(rename = "response")]
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum EventType {
    #[serde(rename = "event")]
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum OkStatus {
    #[serde(rename = "ok")]
    Ok,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ErrorStatus {
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    #[serde(rename = "type")]
    message_type: RequestType,
    pub id: RequestId,
    pub method: String,
    pub params: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessResponse {
    pub version: u32,
    #[serde(rename = "type")]
    message_type: ResponseType,
    pub id: RequestId,
    status: OkStatus,
    pub result: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub version: u32,
    #[serde(rename = "type")]
    message_type: ResponseType,
    pub id: RequestId,
    status: ErrorStatus,
    pub error: RigCtlError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub version: u32,
    #[serde(rename = "type")]
    message_type: EventType,
    pub event: String,
    pub data: JsonObject,
}

impl Event {
    pub fn new(event: impl Into<String>, data: JsonObject) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type: EventType::Event,
            event: event.into(),
            data,
        }
    }
}

impl ErrorResponse {
    pub fn new(id: RequestId, error: RigCtlError) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type: ResponseType::Response,
            id,
            status: ErrorStatus::Error,
            error,
        }
    }
}

impl Request {
    pub fn new(id: RequestId, method: impl Into<String>, params: JsonObject) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type: RequestType::Request,
            id,
            method: method.into(),
            params,
        }
    }
}

impl SuccessResponse {
    pub fn new(id: RequestId, result: JsonObject) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_type: ResponseType::Response,
            id,
            status: OkStatus::Ok,
            result,
        }
    }
}
