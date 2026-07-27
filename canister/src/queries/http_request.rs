use crate::{
    types::http::{
        get_asset_headers, ASSET_ROUTER, HTTP_TREE, IMMUTABLE_ASSET_CACHE_CONTROL,
        NO_CACHE_ASSET_CACHE_CONTROL,
    },
    utils::trace,
};
use bity_ic_canister_logger::LogEntry;
use bity_ic_storage_canister_api::queries::http_request::{
    CallbackFunc, HttpStreamingResponse, StreamingCallbackHttpResponse, StreamingCallbackToken,
    StreamingStrategy,
};
use ic_cdk::api::data_certificate;
use ic_cdk::update;
use ic_cdk_macros::query;
use ic_http_certification::{
    utils::add_v2_certificate_header, DefaultCelBuilder, HttpCertification, HttpCertificationPath,
    HttpCertificationTreeEntry, HttpRequest, HttpResponse, HttpUpdateRequest, HttpUpdateResponse,
    StatusCode, CERTIFICATE_EXPRESSION_HEADER_NAME,
};
use serde_bytes::ByteBuf;

use crate::state::read_state;

/// Bytes carried by a single response or streaming callback.
///
/// The IC caps a message payload at 3 MiB; staying at 2 MiB leaves room for
/// headers and candid framing. Anything larger than this is delivered through
/// the streaming callback, so file size is not bounded by the message cap.
const STREAM_CHUNK_SIZE: usize = 2 * 1024 * 1024;

#[query(hidden = true)]
fn http_request(req: HttpRequest<'static>) -> HttpStreamingResponse {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        "/logs" => serve_logs(bity_ic_canister_logger::export_logs()).into(),
        "/traces" => serve_logs(bity_ic_canister_logger::export_traces()).into(),
        "/metrics" => serve_metrics().into(),
        _ => {
            let is_raw = req
                .headers()
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("host") && v.contains(".raw."));

            if is_raw {
                // Files are served straight from stable memory on the raw
                // domain, streaming anything over the message cap.
                serve_file(&req, &path)
            } else {
                // Files are never served from the asset router. It caps the
                // body at its own chunk size while still advertising the full
                // content length, which is a response browsers reject outright.
                // Everything is funnelled to the raw domain instead, where the
                // streaming path has no size limit.
                if req.headers().to_vec().iter().any(|(k, v)| {
                    k == "referer" && v.contains(ic_cdk::api::canister_self().to_string().as_str())
                }) {
                    not_found()
                } else {
                    HttpResponse::builder().with_upgrade(true).build().into()
                }
            }
        }
    }
}

/// Serves a stored file, streaming anything larger than [STREAM_CHUNK_SIZE].
///
/// No `content-length` header is set: for a streamed response the HTTP gateway
/// assembles the body from the callbacks and determines the length itself.
/// Declaring it here is what made large files unservable before.
fn serve_file(req: &HttpRequest, path: &str) -> HttpStreamingResponse {
    let range_header = req
        .headers()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("range"))
        .map(|(_, v)| v.clone());

    let Some((body, total, content_type)) = read_state(|state| {
        state
            .data
            .storage
            .get_file_slice(path, 0, STREAM_CHUNK_SIZE)
    }) else {
        return not_found();
    };

    let base_headers = || {
        get_asset_headers(vec![
            ("content-type".to_string(), content_type.to_string()),
            (
                "cache-control".to_string(),
                IMMUTABLE_ASSET_CACHE_CONTROL.to_string(),
            ),
            ("accept-ranges".to_string(), "bytes".to_string()),
        ])
    };

    if let Some(range) = range_header {
        if let Some((start, end)) = parse_range_header(&range, total as usize) {
            let len = end - start + 1;
            let Some((slice, _, _)) =
                read_state(|state| state.data.storage.get_file_slice(path, start as u64, len))
            else {
                return not_found();
            };
            let mut headers = base_headers();
            headers.push((
                "content-range".to_string(),
                format!("bytes {}-{}/{}", start, end, total),
            ));
            headers.push(("content-length".to_string(), slice.len().to_string()));
            return HttpStreamingResponse {
                status_code: StatusCode::PARTIAL_CONTENT.as_u16(),
                headers,
                body: ByteBuf::from(slice),
                upgrade: None,
                streaming_strategy: None,
            };
        }
    }

    let streaming_strategy = if (body.len() as u64) < total {
        Some(StreamingStrategy::Callback {
            callback: CallbackFunc::new(
                ic_cdk::api::canister_self(),
                "http_request_streaming_callback".to_string(),
            ),
            token: StreamingCallbackToken {
                path: path.to_string(),
                index: body.len() as u64,
                content_type: content_type.to_string(),
                total_length: total,
            },
        })
    } else {
        None
    };

    HttpStreamingResponse {
        status_code: StatusCode::OK.as_u16(),
        headers: base_headers(),
        body: ByteBuf::from(body),
        upgrade: None,
        streaming_strategy,
    }
}

/// Hands the HTTP gateway the next slice of a file it is already streaming.
#[query(hidden = true)]
fn http_request_streaming_callback(token: StreamingCallbackToken) -> StreamingCallbackHttpResponse {
    let end_of_stream = StreamingCallbackHttpResponse {
        body: ByteBuf::new(),
        token: None,
    };

    let Some((body, total, _)) = read_state(|state| {
        state
            .data
            .storage
            .get_file_slice(&token.path, token.index, STREAM_CHUNK_SIZE)
    }) else {
        return end_of_stream;
    };

    // An empty slice means the offset is at or past the end. Returning a token
    // here would make the gateway call back forever.
    if body.is_empty() {
        return end_of_stream;
    }

    let next_index = token.index.saturating_add(body.len() as u64);
    let next_token = if next_index < total {
        Some(StreamingCallbackToken {
            index: next_index,
            ..token
        })
    } else {
        None
    };

    StreamingCallbackHttpResponse {
        body: ByteBuf::from(body),
        token: next_token,
    }
}

fn not_found() -> HttpStreamingResponse {
    HttpResponse::builder()
        .with_status_code(StatusCode::NOT_FOUND)
        .build()
        .into()
}

fn parse_range_header(range: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 {
        return None;
    }
    let s = range.strip_prefix("bytes=")?;
    let mut parts = s.splitn(2, '-');
    let start: usize = parts.next()?.parse().ok()?;
    let end = match parts.next()? {
        "" => (start + STREAM_CHUNK_SIZE).min(total) - 1,
        e => e.parse::<usize>().ok()?.min(total - 1),
    };
    if start > end || start >= total {
        return None;
    }
    // A client may ask for more than fits in one message; the surplus is served
    // by the follow-up range request the client makes for the remainder.
    let end = end.min(start + STREAM_CHUNK_SIZE - 1);
    Some((start, end))
}

#[update(hidden = true)]
async fn http_request_update(req: HttpUpdateRequest<'static>) -> HttpUpdateResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    // Existence check only. Nothing reads the certified asset router any more,
    // so there is no cache to warm here: pulling the file into the heap would
    // cost memory and buy nothing.
    let file_exists = read_state(|state| state.data.storage.get_file_slice(&path, 0, 0).is_some());
    trace(&format!("http_request_update: {path} exists={file_exists}"));

    match file_exists {
        true => {
            let redirection_url = format!(
                "https://{}.raw.icp0.io{}",
                ic_cdk::api::canister_self().to_string(),
                path.clone()
            );

            let response = HttpResponse::temporary_redirect(
                redirection_url,
                get_asset_headers(vec![
                    (
                        "cache-control".to_string(),
                        NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
                    ),
                    ("content-type".to_string(), "text/plain".to_string()),
                ]),
            )
            .build();
            HttpUpdateResponse::from(response)
        }
        false => {
            let response = HttpResponse::builder()
                .with_status_code(StatusCode::NOT_FOUND)
                .build();
            HttpUpdateResponse::from(response)
        }
    }
}

fn serve_logs(logs: Vec<LogEntry>) -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|_| {
        let body = serde_json::to_vec(&logs).expect("Failed to serialize metrics");
        let headers = get_asset_headers(vec![
            (
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                DefaultCelBuilder::skip_certification().to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
        ]);
        let mut response = HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_body(body)
            .with_headers(headers)
            .build();

        HTTP_TREE.with(|tree| {
            let tree = tree.borrow();

            let metrics_tree_path = HttpCertificationPath::exact("/metrics");
            let metrics_certification = HttpCertification::skip();
            let metrics_tree_entry =
                HttpCertificationTreeEntry::new(&metrics_tree_path, metrics_certification);
            add_v2_certificate_header(
                &data_certificate().expect("No data certificate available"),
                &mut response,
                // witness on the entry we just inserted: provably present
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path(),
            );

            response
        })
    })
}

fn serve_metrics() -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|_| {
        let metrics = read_state(|state| state.metrics());
        let body = serde_json::to_vec(&metrics).expect("Failed to serialize metrics");
        let headers = get_asset_headers(vec![
            (
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                DefaultCelBuilder::skip_certification().to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
        ]);
        let mut response = HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_body(body)
            .with_headers(headers)
            .build();

        HTTP_TREE.with(|tree| {
            let tree = tree.borrow();

            let metrics_tree_path = HttpCertificationPath::exact("/metrics");
            let metrics_certification = HttpCertification::skip();
            let metrics_tree_entry =
                HttpCertificationTreeEntry::new(&metrics_tree_path, metrics_certification);
            add_v2_certificate_header(
                &data_certificate().expect("No data certificate available"),
                &mut response,
                // witness on the entry we just inserted: provably present
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path(),
            );

            response
        })
    })
}
