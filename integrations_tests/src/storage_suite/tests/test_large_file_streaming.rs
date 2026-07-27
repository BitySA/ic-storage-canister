use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use ic_http_gateway::HttpGatewayRequestArgs;

use crate::storage_suite::setup::default_test_setup;
use crate::storage_suite::setup::setup::TestEnv;
use crate::utils::{setup_http_client, upload_file};

/// A file larger than the 3 MiB message cap must still come back whole over the
/// raw domain.
///
/// Before streaming existed, `http_request` put the entire body in one reply and
/// the replica rejected it with
/// `ic0.msg_reply_data_append: application payload size ... cannot be larger than 3145728`,
/// which reaches the browser as a 503. Ten production files were dead this way.
#[test]
fn test_large_file_served_whole_over_raw() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    // 6,205,837 bytes: three streaming chunks plus a remainder.
    let file_path = "./src/storage_suite/assets/test.png";
    let upload_path = "/big.png";

    let buffer = upload_file(pic, controller, storage_canister_id, file_path, upload_path)
        .expect("Upload failed");
    assert!(
        buffer.len() > 3 * 1024 * 1024,
        "fixture must exceed the message cap, got {} bytes",
        buffer.len()
    );

    let (rt, http_gateway) = setup_http_client(pic);

    let response = rt.block_on(async {
        let mut request = http_gateway.request(HttpGatewayRequestArgs {
            canister_id: storage_canister_id,
            canister_request: Request::builder()
                .uri(upload_path)
                // The gateway test client does not derive a host from the
                // uri, so the raw domain has to be stated explicitly.
                .header(
                    "host",
                    format!("{}.raw.icp0.io", storage_canister_id.to_text()),
                )
                .body(Bytes::new())
                .unwrap(),
        });
        // The real boundary node does not verify certification on the raw
        // domain; the test client has to be told the same.
        request.unsafe_set_skip_verification(true);
        request.send().await
    });

    assert_eq!(
        response.canister_response.status(),
        200,
        "large file must not trap"
    );

    let body = rt.block_on(async {
        response
            .canister_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    });

    assert_eq!(
        body.len(),
        buffer.len(),
        "streamed body was truncated: got {} of {} bytes",
        body.len(),
        buffer.len()
    );
    assert_eq!(body, buffer, "streamed body does not match the upload");
}

/// A range request on a large file must be honoured and must stay under the
/// message cap even when the client asks for more than fits in one reply.
#[test]
fn test_large_file_range_request_is_clamped() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        ..
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let upload_path = "/ranged.png";

    let buffer = upload_file(pic, controller, storage_canister_id, file_path, upload_path)
        .expect("Upload failed");

    let (rt, http_gateway) = setup_http_client(pic);

    // Ask for the whole file as a single range; the canister must clamp the
    // slice rather than trap.
    let response = rt.block_on(async {
        let mut request = http_gateway.request(HttpGatewayRequestArgs {
            canister_id: storage_canister_id,
            canister_request: Request::builder()
                .uri(upload_path)
                .header(
                    "host",
                    format!("{}.raw.icp0.io", storage_canister_id.to_text()),
                )
                .header("range", format!("bytes=0-{}", buffer.len() - 1))
                .body(Bytes::new())
                .unwrap(),
        });
        // The real boundary node does not verify certification on the raw
        // domain; the test client has to be told the same.
        request.unsafe_set_skip_verification(true);
        request.send().await
    });

    assert_eq!(response.canister_response.status(), 206);

    let body = rt.block_on(async {
        response
            .canister_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    });

    assert!(
        !body.is_empty() && body.len() <= 2 * 1024 * 1024,
        "range slice must be non-empty and within the message cap, got {}",
        body.len()
    );
    assert_eq!(
        body,
        buffer[..body.len()].to_vec(),
        "range slice does not match the start of the upload"
    );
}
