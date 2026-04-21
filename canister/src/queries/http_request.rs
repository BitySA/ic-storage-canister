use crate::{
    state::mutate_state,
    types::http::{
        get_asset_headers, ASSET_ROUTER, HTTP_TREE, IMMUTABLE_ASSET_CACHE_CONTROL,
        NO_CACHE_ASSET_CACHE_CONTROL,
    },
    utils::trace,
};
use bity_ic_canister_logger::LogEntry;
use ic_cdk::api::data_certificate;
use ic_cdk::update;
use ic_cdk_macros::query;
use ic_http_certification::{
    utils::add_v2_certificate_header, DefaultCelBuilder, HttpCertification, HttpCertificationPath,
    HttpCertificationTreeEntry, HttpRequest, HttpResponse, HttpUpdateRequest, HttpUpdateResponse,
    StatusCode, CERTIFICATE_EXPRESSION_HEADER_NAME,
};

use crate::state::read_state;

#[query(hidden = true)]
async fn http_request(req: HttpRequest<'static>) -> HttpResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        "/logs" => serve_logs(bity_ic_canister_logger::export_logs()),
        "/traces" => serve_logs(bity_ic_canister_logger::export_traces()),
        "/metrics" => serve_metrics(),
        _ => {
            let asset_resp = serve_asset(&req);
            trace(&format!("asset_resp: {:?}", asset_resp));

            // FIXME: check which domain
            match asset_resp {
                Some(response) => response,
                None => {
                    let is_raw = req
                        .headers()
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case("host") && v.contains(".raw."));

                    if is_raw {
                        serve_from_stable_memory(&req, &path)
                    } else if req.headers().to_vec().iter().any(|(k, v)| {
                        k == "referer"
                            && v.contains(ic_cdk::api::canister_self().to_string().as_str())
                    }) {
                        HttpResponse::builder()
                            .with_status_code(StatusCode::NOT_FOUND)
                            .build()
                    } else {
                        HttpResponse::builder().with_upgrade(true).build()
                    }
                }
            }
        }
    }
}

fn parse_range_header(range: &str, total: usize) -> Option<(usize, usize)> {
    if total == 0 {
        return None;
    }
    let s = range.strip_prefix("bytes=")?;
    let mut parts = s.splitn(2, '-');
    let start: usize = parts.next()?.parse().ok()?;
    let end = match parts.next()? {
        "" => (start + 2 * 1024 * 1024).min(total) - 1,
        e => e.parse::<usize>().ok()?.min(total - 1),
    };
    if start > end || start >= total {
        return None;
    }
    Some((start, end))
}

fn serve_from_stable_memory(req: &HttpRequest, path: &str) -> HttpResponse<'static> {
    let result = read_state(|state| state.data.storage.get_file_data(path));

    match result {
        None => HttpResponse::builder()
            .with_status_code(StatusCode::NOT_FOUND)
            .build(),
        Some((data, content_type)) => {
            let range_header = req
                .headers()
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("range"))
                .map(|(_, v)| v.clone());

            let total = data.len();
            let headers = get_asset_headers(vec![
                ("content-type".to_string(), content_type.to_string()),
                (
                    "cache-control".to_string(),
                    IMMUTABLE_ASSET_CACHE_CONTROL.to_string(),
                ),
                ("accept-ranges".to_string(), "bytes".to_string()),
            ]);

            if let Some(range) = range_header {
                if let Some((start, end)) = parse_range_header(&range, total) {
                    let body = data[start..=end].to_vec();
                    return HttpResponse::builder()
                        .with_status_code(StatusCode::PARTIAL_CONTENT)
                        .with_headers({
                            let mut h = headers;
                            h.push((
                                "content-range".to_string(),
                                format!("bytes {}-{}/{}", start, end, total),
                            ));
                            h
                        })
                        .with_body(body)
                        .build();
                }
            }

            HttpResponse::builder()
                .with_status_code(StatusCode::OK)
                .with_headers(headers)
                .with_body(data)
                .build()
        }
    }
}

#[update(hidden = true)]
async fn http_request_update(req: HttpUpdateRequest<'static>) -> HttpUpdateResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        _ => {
            trace("Cache miss");
            let cache_miss_ret =
                mutate_state(|state| state.data.storage.cache_miss(&state.env, path.clone()));
            match cache_miss_ret {
                Ok(_) => {
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
                Err(_) => {
                    let response = HttpResponse::builder()
                        .with_status_code(StatusCode::NOT_FOUND)
                        .build();
                    HttpUpdateResponse::from(response)
                }
            }
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
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path(),
            );

            response
        })
    })
}

fn serve_asset(req: &HttpRequest) -> Option<HttpResponse<'static>> {
    ASSET_ROUTER.with_borrow(|asset_router| {
        let data_cert = data_certificate().expect("No data certificate available");

        if let Ok(response) = asset_router.serve_asset(&data_cert, &req) {
            Some(response)
        } else {
            None
        }
    })
}
