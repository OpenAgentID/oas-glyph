//! Integration tests for the HTTP API using axum's test utilities.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn app() -> axum::Router {
    oas_glyph::api::router()
}

// --- /healthz ---

#[tokio::test]
async fn healthz_returns_json_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], "0.1.0");
    assert!(json["uptime_secs"].is_number());
}

// --- /livez ---

#[tokio::test]
async fn livez_returns_ok_with_glyph_bytes() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/livez")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["glyph_bytes"].as_u64().unwrap() > 0);
}

// --- POST /v1/glyph/generate ---

#[tokio::test]
async fn generate_glyph_svg() {
    let body = serde_json::json!({
        "did": "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
        "format": "svg"
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/glyph/generate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["kind"], "agent");
    assert_eq!(json["namespace"], "l1fe");
    assert_eq!(json["format"], "svg");
    let svg_data = json["data"].as_str().unwrap();
    assert!(svg_data.starts_with("<svg"));
}

#[tokio::test]
async fn generate_glyph_png() {
    let body = serde_json::json!({
        "did": "did:oas:l1fe:hmr:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp",
        "format": "png",
        "size": 128
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/glyph/generate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["format"], "png");
    // Data should be base64-encoded
    let data = json["data"].as_str().unwrap();
    assert!(!data.is_empty());
}

#[tokio::test]
async fn generate_invalid_did_returns_error_envelope() {
    let body = serde_json::json!({
        "did": "not-a-valid-did"
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/glyph/generate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "INVALID_DID");
    assert!(json["error"]["message"].as_str().unwrap().len() > 0);
}

// --- GET /v1/glyph/svg/:did ---

#[tokio::test]
async fn get_glyph_svg_by_path() {
    let did_encoded = urlencoding::encode(
        "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    );

    let resp = app()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/glyph/svg/{}", did_encoded))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert_eq!(ct, "image/svg+xml");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let svg = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(svg.starts_with("<svg"));
}

// --- GET /v1/glyph/png/:did ---

#[tokio::test]
async fn get_glyph_png_by_path() {
    let did_encoded = urlencoding::encode(
        "did:oas:l1fe:tool:z6MknGc3ocHs3zdPiJbnaaqDi5RNw3A6tDfHPbp7ihKBpKG3",
    );

    let resp = app()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/glyph/png/{}?size=64", did_encoded))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert_eq!(ct, "image/png");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    // PNG magic bytes
    assert_eq!(bytes[0], 0x89);
    assert_eq!(bytes[1], 0x50);
}

// --- GET /v1/palette/:did ---

#[tokio::test]
async fn get_palette_valid_did() {
    let did_encoded = urlencoding::encode(
        "did:oas:l1fe:service:z6Mkp9Juh5gHzRdNYW2gcCRZPW2iB2rvwyVU7VHtohmBCH8z",
    );

    let resp = app()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/palette/{}", did_encoded))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["primary"].as_str().unwrap().starts_with('#'));
    assert!(json["background"].as_str().unwrap().starts_with('#'));
}

#[tokio::test]
async fn get_palette_invalid_did_returns_error() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/v1/palette/baddata")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["error"]["code"].as_str().is_some());
}

// --- POST /v1/glyph/info ---

#[tokio::test]
async fn glyph_info_valid_did() {
    let body = serde_json::json!({
        "did": "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    });

    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/glyph/info")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["grid_size"], 12);
    assert!(json["data_cells"].as_u64().unwrap() > 80);
    assert!(json["payload_bytes"].as_u64().unwrap() > 0);
    assert!(json["ecc_bytes"].as_u64().unwrap() > 0);
}
