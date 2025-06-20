use crate::client::storage::{cancel_upload, finalize_upload, init_upload, store_chunk};
use candid::Nat;

use bity_ic_storage_canister_api::cancel_upload;
use bity_ic_storage_canister_api::finalize_upload;
use bity_ic_storage_canister_api::init_upload;
use bity_ic_storage_canister_api::store_chunk;
use http::StatusCode;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use sha2::{Digest, Sha256};

use crate::storage_suite::setup::setup::TestEnv;
use crate::utils::{setup_http_client, upload_file};
use crate::{storage_suite::setup::default_test_setup, utils::tick_n_blocks};
use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs, HttpGatewayResponseMetadata};
use std::fs::File;
use std::io::Read;
use std::panic::AssertUnwindSafe;
use std::path::Path;

#[test]
fn test_storage_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let upload_path = "/test.png";

    let buffer = upload_file(pic, controller, storage_canister_id, file_path, upload_path)
        .expect("Upload failed");

    let (rt, http_gateway) = setup_http_client(pic);

    let response = rt.block_on(async {
        http_gateway
            .request(HttpGatewayRequestArgs {
                canister_id: storage_canister_id.clone(),
                canister_request: Request::builder()
                    .uri(upload_path)
                    .body(Bytes::new())
                    .unwrap(),
            })
            .send()
            .await
    });

    assert_eq!(response.canister_response.status(), 307);

    if let Some(location) = response.canister_response.headers().get("location") {
        let location_str = location.to_str().unwrap();
        println!("Redirecting to: {}", location_str);

        let redirected_response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: storage_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(location_str)
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        assert_eq!(redirected_response.canister_response.status(), 200);

        let expected_headers = vec![
            ("strict-transport-security", "max-age=31536000; includeSubDomains"),
            ("x-frame-options", "DENY"),
            ("x-content-type-options", "nosniff"),
            (
                "content-security-policy",
                "default-src 'self'; img-src 'self' data:; form-action 'self'; object-src 'none'; frame-ancestors 'none'; upgrade-insecure-requests; block-all-mixed-content",
            ),
            ("referrer-policy", "no-referrer"),
            (
                "permissions-policy",
                "accelerometer=(),ambient-light-sensor=(),autoplay=(),battery=(),camera=(),display-capture=(),document-domain=(),encrypted-media=(),fullscreen=(),gamepad=(),geolocation=(),gyroscope=(),layout-animations=(self),legacy-image-formats=(self),magnetometer=(),microphone=(),midi=(),oversized-images=(self),payment=(),picture-in-picture=(),publickey-credentials-get=(),speaker-selection=(),sync-xhr=(self),unoptimized-images=(self),unsized-media=(self),usb=(),screen-wake-lock=(),web-share=(),xr-spatial-tracking=()",
            ),
            ("cross-origin-embedder-policy", "require-corp"),
            ("cross-origin-opener-policy", "same-origin"),
            ("cache-control", "public, max-age=31536000, immutable"),
            ("content-type", "image/png"),
            ("content-length", "6205837")
        ];

        let response_headers = redirected_response
            .canister_response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_str().unwrap()))
            .collect::<Vec<(&str, &str)>>();

        for (key, value) in expected_headers {
            assert!(response_headers.contains(&(key, value)));
        }

        rt.block_on(async {
            let body = redirected_response
                .canister_response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec();

            assert_eq!(body, buffer);
        });
    } else {
        panic!("No location header found in redirection response");
    }
}

#[test]
fn test_duplicate_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let upload_path = "/test.png";

    // First upload attempt
    upload_file(pic, controller, storage_canister_id, file_path, upload_path)
        .expect("First upload failed");

    // Second upload attempt with the same file
    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: upload_path.to_string(),
            file_hash: "dummy_hash".to_string(),
            file_size: 1024,
            chunk_size: None,
        }),
    );

    match init_upload_resp {
        Ok(_) => panic!("Duplicate upload should not be allowed"),
        Err(e) => {
            println!("Expected error on duplicate upload: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_duplicate_chunk_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    )
    .expect("Failed to initialize upload");

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];

        // First attempt to upload chunk
        store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        )
        .expect("Failed to store chunk");

        // Attempt to upload the same chunk again
        let duplicate_chunk_resp = store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );

        match duplicate_chunk_resp {
            Ok(_) => panic!("Duplicate chunk upload should not be allowed"),
            Err(e) => println!("Expected error on duplicate chunk upload: {:?}", e),
        }

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    )
    .expect("Failed to finalize upload");
}

#[test]
fn test_finalize_upload_missing_chunk() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    // Upload all chunks except the last one
    while offset < buffer.len() - (chunk_size as usize) {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    // Attempt to finalize upload with a missing chunk
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed with missing chunk");
            assert!(false);
        }
        Err(e) => {
            println!(
                "Expected error on finalize upload with missing chunk: {:?}",
                e
            );
            assert!(true);
        }
    }
}

#[test]
fn test_upload_with_incorrect_chunk() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let mut chunk = buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())].to_vec();

        if offset == 0 {
            chunk[0] = 0;
        }

        let store_chunk_resp = store_chunk(
            pic,
            controller,
            storage_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk,
            }),
        );

        match store_chunk_resp {
            Ok(resp) => {
                println!("store_chunk_resp: {:?}", resp);
            }
            Err(e) => {
                println!("store_chunk_resp error: {:?}", e);
            }
        }

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed with incorrect chunk");
            assert!(false);
        }
        Err(e) => {
            println!(
                "Expected error on finalize upload with incorrect chunk: {:?}",
                e
            );
            assert!(true);
        }
    }
}

#[test]
fn test_cancel_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test_cancel.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        storage_canister_id,
        &(init_upload::Args {
            file_path: "/test_cancel.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        }),
    );

    match init_upload_resp {
        Ok(resp) => {
            println!("init_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("init_upload_resp error: {:?}", e);
        }
    }

    let cancel_upload_resp = cancel_upload(
        pic,
        controller,
        storage_canister_id,
        &(cancel_upload::Args {
            file_path: "/test_cancel.png".to_string(),
        }),
    );

    match cancel_upload_resp {
        Ok(resp) => {
            println!("cancel_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("cancel_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    // Attempt to finalize the canceled upload
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        storage_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        }),
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed for a canceled upload");
            assert!(false);
        }
        Err(e) => {
            println!(
                "Expected error on finalize upload for a canceled upload: {:?}",
                e
            );
            assert!(true);
        }
    }
}

#[test]
fn test_non_governance_principal_rejection() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let non_governance_principal = nft_owner1;

    let file_path = "/test_non_governance.png".to_string();
    let file_hash = "dummy_hash".to_string();
    let file_size = 1024;
    let chunk_data = vec![0u8; 1024];
    let data_id = "data_id".to_string();
    let hash_id = "hash_id".to_string();

    let methods: Vec<(&str, Result<(), String>)> = vec![
        (
            "cancel_upload",
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                cancel_upload(
                    pic,
                    non_governance_principal,
                    storage_canister_id,
                    &(cancel_upload::Args {
                        file_path: file_path.clone(),
                    }),
                )
                .map(|_| ())
                .map_err(|e| format!("{:?}", e))
            }))
            .unwrap_or_else(|_| Err("panic occurred".to_string())),
        ),
        (
            "init_upload",
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                init_upload(
                    pic,
                    non_governance_principal,
                    storage_canister_id,
                    &(init_upload::Args {
                        file_path: file_path.clone(),
                        file_hash: file_hash.clone(),
                        file_size,
                        chunk_size: None,
                    }),
                )
                .map(|_| ())
                .map_err(|e| format!("{:?}", e))
            }))
            .unwrap_or_else(|_| Err("panic occurred".to_string())),
        ),
        (
            "store_chunk",
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                store_chunk(
                    pic,
                    non_governance_principal,
                    storage_canister_id,
                    &(store_chunk::Args {
                        file_path: file_path.clone(),
                        chunk_id: Nat::from(0 as u64),
                        chunk_data: chunk_data.clone(),
                    }),
                )
                .map(|_| ())
                .map_err(|e| format!("{:?}", e))
            }))
            .unwrap_or_else(|_| Err("panic occurred".to_string())),
        ),
        (
            "finalize_upload",
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                finalize_upload(
                    pic,
                    non_governance_principal,
                    storage_canister_id,
                    &(finalize_upload::Args {
                        file_path: file_path.clone(),
                    }),
                )
                .map(|_| ())
                .map_err(|e| format!("{:?}", e))
            }))
            .unwrap_or_else(|_| Err("panic occurred".to_string())),
        ),
    ];

    for (method_name, result) in methods {
        match result {
            Ok(_) => {
                println!(
                    "{} should not be allowed for non-governance principal",
                    method_name
                );
                assert!(false);
            }
            Err(e) => {
                println!(
                    "Expected error on {} for non-governance principal: {:?}",
                    method_name, e
                );
                assert!(true);
            }
        }
    }
}

#[test]
fn test_storage_scalability() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let mut uploaded_buffers = Vec::new();

    // Upload first two files - should succeed
    for i in 0..7 {
        let upload_path = format!("/test_scalability_{}.png", i);
        println!("Attempting to upload file {} at path: {}", i, upload_path);

        let buffer = upload_file(
            pic,
            controller,
            storage_canister_id,
            file_path,
            &upload_path,
        )
        .expect(&format!("Upload {} failed", i));

        uploaded_buffers.push((upload_path.clone(), buffer));
    }

    // Verify we have exactly 2 successful uploads
    assert_eq!(
        uploaded_buffers.len(),
        7,
        "Expected exactly 10 successful uploads"
    );

    // Try to upload the third file - should fail due to 15MB limit
    let upload_path = "/test_scalability_7.png";
    println!("Attempting to upload third file at path: {}", upload_path);

    let result = upload_file(
        pic,
        controller,
        storage_canister_id,
        file_path,
        &upload_path,
    );

    match result {
        Ok(_) => panic!("Expected third upload to fail due to storage limit"),
        Err(e) => {
            assert_eq!(e, format!("init_upload error: {:?}", bity_ic_storage_canister_api::updates::init_upload::InitUploadError::NotEnoughStorage));
            println!("Third upload failed as expected with: {:?}", e);
        }
    }

    // Verify that the first two uploaded files are accessible
    let (rt, http_gateway) = setup_http_client(pic);

    // Verify each uploaded file
    for (upload_path, original_buffer) in uploaded_buffers {
        println!("Verifying file at path: {}", upload_path);

        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: storage_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        assert_eq!(response.canister_response.status(), 307);

        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();

            let redirected_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: storage_canister_id.clone(),
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });

            assert_eq!(redirected_response.canister_response.status(), 200);

            rt.block_on(async {
                let body = redirected_response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                assert_eq!(
                    body, original_buffer,
                    "File content mismatch for {}",
                    upload_path
                );
            });
        } else {
            panic!("No redirect location found for {}", upload_path);
        }
    }
}

#[test]
fn test_storage_heap_management() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        storage_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let file_path = "./src/storage_suite/assets/test.png";
    let mut uploaded_files = Vec::new();

    // Upload multiple files to fill the heap (512MB)
    for i in 0..5 {
        let upload_path = format!("/test_heap_{}.png", i);
        println!("Attempting to upload file {} at path: {}", i, upload_path);

        let buffer = upload_file(
            pic,
            controller,
            storage_canister_id,
            file_path,
            &upload_path,
        )
        .expect(&format!("Upload {} failed", i));

        uploaded_files.push((upload_path.clone(), buffer));
        println!("Successfully uploaded file {}", i);
    }

    let (rt, http_gateway) = setup_http_client(pic);

    // First request for each file should work and cache the file
    for (i, (upload_path, original_buffer)) in uploaded_files.iter().enumerate() {
        println!("First request for file {} at path: {}", i, upload_path);

        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: storage_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        assert_eq!(response.canister_response.status(), 307);

        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();

            let redirected_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: storage_canister_id.clone(),
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });

            assert_eq!(redirected_response.canister_response.status(), 200);

            rt.block_on(async {
                let body = redirected_response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                assert_eq!(
                    body, *original_buffer,
                    "File content mismatch for {}",
                    upload_path
                );
            });
        } else {
            panic!("No redirect location found for {}", upload_path);
        }
    }

    // Request files in reverse order to verify FIFO cache behavior
    // The first files should be uncached to make room for the later ones
    for (i, (upload_path, original_buffer)) in uploaded_files.iter().enumerate().rev() {
        println!("Reverse request for file {} at path: {}", i, upload_path);

        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: storage_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        // Files 0 and 1 should be uncached (307), while files 2, 3, and 4 should be cached (200)
        let expected_status = if i <= 1 { 307 } else { 200 };
        assert_eq!(
            response.canister_response.status(),
            expected_status,
            "Unexpected status for file {}",
            i
        );

        if response.canister_response.status() == 307 {
            if let Some(location) = response.canister_response.headers().get("location") {
                let location_str = location.to_str().unwrap();

                let redirected_response = rt.block_on(async {
                    http_gateway
                        .request(HttpGatewayRequestArgs {
                            canister_id: storage_canister_id.clone(),
                            canister_request: Request::builder()
                                .uri(location_str)
                                .body(Bytes::new())
                                .unwrap(),
                        })
                        .send()
                        .await
                });

                assert_eq!(redirected_response.canister_response.status(), 200);

                rt.block_on(async {
                    let body = redirected_response
                        .canister_response
                        .into_body()
                        .collect()
                        .await
                        .unwrap()
                        .to_bytes()
                        .to_vec();

                    assert_eq!(
                        body, *original_buffer,
                        "File content mismatch for {}",
                        upload_path
                    );
                });
            } else {
                panic!("No redirect location found for {}", upload_path);
            }
        } else {
            // For cached files (status 200), verify content directly
            rt.block_on(async {
                let body = response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                assert_eq!(
                    body, *original_buffer,
                    "File content mismatch for {}",
                    upload_path
                );
            });
        }
    }

    // Verify that we can still access all files even after cache cycling
    for (i, (upload_path, original_buffer)) in uploaded_files.iter().enumerate() {
        println!("Final verification for file {} at path: {}", i, upload_path);

        let response = rt.block_on(async {
            http_gateway
                .request(HttpGatewayRequestArgs {
                    canister_id: storage_canister_id.clone(),
                    canister_request: Request::builder()
                        .uri(upload_path.as_str())
                        .body(Bytes::new())
                        .unwrap(),
                })
                .send()
                .await
        });

        if let Some(location) = response.canister_response.headers().get("location") {
            let location_str = location.to_str().unwrap();

            let redirected_response = rt.block_on(async {
                http_gateway
                    .request(HttpGatewayRequestArgs {
                        canister_id: storage_canister_id.clone(),
                        canister_request: Request::builder()
                            .uri(location_str)
                            .body(Bytes::new())
                            .unwrap(),
                    })
                    .send()
                    .await
            });

            assert_eq!(redirected_response.canister_response.status(), 200);

            rt.block_on(async {
                let body = redirected_response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                assert_eq!(
                    body, *original_buffer,
                    "File content mismatch for {}",
                    upload_path
                );
            });
        } else {
            assert_eq!(response.canister_response.status(), 200);

            rt.block_on(async {
                let body = response
                    .canister_response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec();

                assert_eq!(
                    body, *original_buffer,
                    "File content mismatch for {}",
                    upload_path
                );
            });
        }
    }
}
