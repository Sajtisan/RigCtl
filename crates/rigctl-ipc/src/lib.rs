mod error;
mod message;

pub use error::RigCtlError;

pub use message::{
    ErrorResponse, Event, JsonObject, PROTOCOL_VERSION, Request, RequestId, Response,
    SuccessResponse,
};
