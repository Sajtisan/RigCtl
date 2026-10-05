use rigctl_ipc::{
    ErrorResponse, Event, PROTOCOL_VERSION, Request, Response, RigCtlError, SuccessResponse,
};
use serde_json::{Map, json};

#[test]
fn protocol_version_is_one() {
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn request_matches_v1_wire_format() {
    let mut params = Map::new();
    params.insert("dpi".into(), json!(800));

    let request = Request::new(42, "mouse.dpi.set", params);

    let value = serde_json::to_value(&request).unwrap();

    assert_eq!(
        value,
        json!({
            "version": 1,
            "type": "request",
            "id": 42,
            "method": "mouse.dpi.set",
            "params": {
                "dpi": 800
            }
        })
    );

    let decoded: Request = serde_json::from_value(value).unwrap();

    assert_eq!(decoded, request);
}

#[test]
fn success_response_matches_v1_wire_format() {
    let mut result = Map::new();
    result.insert("dpi".into(), json!(800));

    let response = SuccessResponse::new(42, result);

    let value = serde_json::to_value(&response).unwrap();

    assert_eq!(
        value,
        json!({
            "version": 1,
            "type": "response",
            "id": 42,
            "status": "ok",
            "result": {
                "dpi": 800
            }
        })
    );

    let decoded: SuccessResponse = serde_json::from_value(value).unwrap();

    assert_eq!(decoded, response);
}

#[test]
fn error_response_matches_v1_wire_format() {
    let error = RigCtlError::new("unsupported_value", "Unsupported polling rate");

    let response = ErrorResponse::new(42, error);

    let value = serde_json::to_value(&response).unwrap();

    assert_eq!(
        value,
        json!({
            "version": 1,
            "type": "response",
            "id": 42,
            "status": "error",
            "error": {
                "code": "unsupported_value",
                "message": "Unsupported polling rate"
            }
        })
    );

    let decoded: ErrorResponse = serde_json::from_value(value).unwrap();

    assert_eq!(decoded, response);
}

#[test]
fn event_matches_v1_wire_format() {
    let mut data = Map::new();
    data.insert("percent".into(), json!(72));

    let event = Event::new("mouse.battery.changed", data);

    let value = serde_json::to_value(&event).unwrap();

    assert_eq!(
        value,
        json!({
            "version": 1,
            "type": "event",
            "event": "mouse.battery.changed",
            "data": {
                "percent": 72
            }
        })
    );

    let decoded: Event = serde_json::from_value(value).unwrap();

    assert_eq!(decoded, event);
}

#[test]
fn response_cannot_contain_result_and_error() {
    let value = json!({
        "version": 1,
        "type": "response",
        "id": 42,
        "status": "ok",
        "result": {
            "dpi": 800
        },
        "error": {
            "code": "internal_error",
            "message": "this should not coexist with result"
        }
    });

    assert!(serde_json::from_value::<Response>(value).is_err());
}

#[test]
fn request_params_must_be_an_object() {
    let value = json!({
        "version": 1,
        "type": "request",
        "id": 42,
        "method": "mouse.dpi.set",
        "params": [800]
    });

    assert!(serde_json::from_value::<Request>(value).is_err());
}
