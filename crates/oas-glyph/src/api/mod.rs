//! # HTTP API
//!
//! RESTful API for generating and scanning OpenAgent Glyphs.
//!
//! ## Endpoints
//!
//! - `POST /v1/glyph/generate` — Generate a glyph from a DID
//! - `POST /v1/glyph/scan` — Scan a glyph image to recover the DID
//! - `GET /v1/glyph/:did` — Generate and return a glyph as SVG/PNG
//! - `GET /v1/palette/:did` — Return the identity-derived color palette
//! - `GET /healthz` — Health check (JSON status)
//! - `GET /livez` — Liveness probe (generates a test glyph)

use axum::{
    extract::{Path, Query},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::visual::palette::GlyphPalette;
use crate::visual::renderer::{did_to_png, did_to_svg, RenderConfig};

/// Standard JSON error envelope returned by all API endpoints.
#[derive(Debug, Serialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

/// Error body within the JSON error envelope.
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

/// Typed API error with status code and structured JSON body.
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    fn bad_request(code: &str, message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: code.to_string(),
            message,
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR".to_string(),
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiErrorEnvelope {
            error: ApiErrorBody {
                code: self.code,
                message: self.message,
            },
        };
        (self.status, Json(body)).into_response()
    }
}

/// Build a CORS layer from the `ALLOWED_ORIGINS` env var.
///
/// If `ALLOWED_ORIGINS` is set, it is treated as a comma-separated list of
/// allowed origin strings (e.g. `http://localhost:3000,https://app.example.com`).
///
/// If unset, the default permits any `http://localhost:*` origin (development mode).
fn build_cors_layer() -> CorsLayer {
    use axum::http::Method;

    let allowed_methods = [Method::GET, Method::POST, Method::OPTIONS];

    match std::env::var("ALLOWED_ORIGINS") {
        Ok(origins_str) if !origins_str.is_empty() => {
            let origins: Vec<HeaderValue> = origins_str
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    HeaderValue::from_str(trimmed).ok()
                })
                .collect();

            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(allowed_methods)
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
        _ => {
            // Development default: allow localhost on any port
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin, _| {
                    origin
                        .to_str()
                        .map(|s| s.starts_with("http://localhost"))
                        .unwrap_or(false)
                }))
                .allow_methods(allowed_methods)
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
    }
}

/// Startup instant for uptime tracking.
static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Initialize the start time. Must be called before the router is used.
pub fn init_start_time() {
    START_TIME.get_or_init(std::time::Instant::now);
}

/// Build the API router.
pub fn router() -> Router {
    init_start_time();

    Router::new()
        .route("/healthz", get(healthz))
        .route("/livez", get(livez))
        .route("/v1/glyph/generate", post(generate_glyph))
        .route("/v1/glyph/svg/:did", get(get_glyph_svg))
        .route("/v1/glyph/png/:did", get(get_glyph_png))
        .route("/v1/palette/:did", get(get_palette))
        .route("/v1/glyph/info", post(glyph_info))
        .layer(build_cors_layer())
}

// --- Request/Response types ---

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub did: String,
    #[serde(default)]
    pub size: Option<u32>,
    #[serde(default)]
    pub format: Option<String>, // "svg" or "png"
    #[serde(default)]
    pub padding: Option<u32>,
    #[serde(default)]
    pub corner_radius: Option<u32>,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub did: String,
    pub kind: String,
    pub namespace: String,
    pub format: String,
    /// Base64-encoded image data (for PNG) or raw SVG string
    pub data: String,
    pub palette: PaletteResponse,
}

#[derive(Serialize)]
pub struct PaletteResponse {
    pub background: String,
    pub primary: String,
    pub secondary: String,
    pub data_colors: [String; 4],
    pub finder_dark: String,
    pub finder_light: String,
    pub kind_accent: String,
}

impl From<&GlyphPalette> for PaletteResponse {
    fn from(p: &GlyphPalette) -> Self {
        Self {
            background: p.background.to_hex(),
            primary: p.primary.to_hex(),
            secondary: p.secondary.to_hex(),
            data_colors: [
                p.data_colors[0].to_hex(),
                p.data_colors[1].to_hex(),
                p.data_colors[2].to_hex(),
                p.data_colors[3].to_hex(),
            ],
            finder_dark: p.finder_dark.to_hex(),
            finder_light: p.finder_light.to_hex(),
            kind_accent: p.kind_accent.to_hex(),
        }
    }
}

#[derive(Serialize)]
pub struct GlyphInfoResponse {
    pub did: String,
    pub kind: String,
    pub namespace: String,
    pub identifier: String,
    pub grid_size: usize,
    pub data_cells: usize,
    pub payload_bytes: usize,
    pub ecc_bytes: usize,
    pub total_codeword_bytes: usize,
    pub error_correction_percent: f64,
}

#[derive(Deserialize)]
pub struct GlyphInfoRequest {
    pub did: String,
}

#[derive(Deserialize)]
pub struct RenderQuery {
    pub size: Option<u32>,
    pub padding: Option<u32>,
}

// --- Health / liveness ---

/// Health check response with structured JSON.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

/// Liveness probe response.
#[derive(Serialize)]
pub struct LivezResponse {
    pub status: String,
    pub glyph_bytes: usize,
}

// --- Handlers ---

async fn healthz() -> Json<HealthResponse> {
    let uptime = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);

    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
        uptime_secs: uptime,
    })
}

/// Liveness probe: generates a known test glyph and confirms output is non-empty.
async fn livez() -> std::result::Result<Json<LivezResponse>, ApiError> {
    let test_did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    let svg = did_to_svg(test_did, None)
        .map_err(|e| ApiError::internal(format!("liveness glyph failed: {}", e)))?;

    if svg.is_empty() {
        return Err(ApiError::internal(
            "liveness glyph produced empty output".to_string(),
        ));
    }

    Ok(Json(LivezResponse {
        status: "ok".to_string(),
        glyph_bytes: svg.len(),
    }))
}

async fn generate_glyph(
    Json(req): Json<GenerateRequest>,
) -> std::result::Result<Json<GenerateResponse>, ApiError> {
    let parsed = crate::ParsedDid::parse(&req.did)
        .map_err(|e| ApiError::bad_request("INVALID_DID", e.to_string()))?;

    let palette = GlyphPalette::from_did(&req.did);
    let format = req.format.unwrap_or_else(|| "svg".into());

    let mut config = RenderConfig::default();
    if let Some(s) = req.size {
        config.size = s;
    }
    if let Some(p) = req.padding {
        config.padding = p;
    }
    if let Some(r) = req.corner_radius {
        config.corner_radius = r;
    }

    let data = match format.as_str() {
        "png" => {
            let png_bytes =
                did_to_png(&req.did, config.size).map_err(|e| ApiError::internal(e.to_string()))?;
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(&png_bytes)
        }
        _ => did_to_svg(&req.did, Some(config)).map_err(|e| ApiError::internal(e.to_string()))?,
    };

    Ok(Json(GenerateResponse {
        did: req.did,
        kind: parsed.kind.as_str().to_string(),
        namespace: parsed.namespace.clone(),
        format,
        data,
        palette: PaletteResponse::from(&palette),
    }))
}

async fn get_glyph_svg(
    Path(did): Path<String>,
    Query(query): Query<RenderQuery>,
) -> std::result::Result<Response, ApiError> {
    // URL-decode the DID (colons may be encoded)
    let did = urlencoding::decode(&did)
        .map_err(|e| ApiError::bad_request("INVALID_ENCODING", e.to_string()))?
        .to_string();

    let mut config = RenderConfig::default();
    if let Some(s) = query.size {
        config.size = s;
    }
    if let Some(p) = query.padding {
        config.padding = p;
    }

    let svg = did_to_svg(&did, Some(config))
        .map_err(|e| ApiError::bad_request("INVALID_DID", e.to_string()))?;

    Ok(([(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response())
}

async fn get_glyph_png(
    Path(did): Path<String>,
    Query(query): Query<RenderQuery>,
) -> std::result::Result<Response, ApiError> {
    let did = urlencoding::decode(&did)
        .map_err(|e| ApiError::bad_request("INVALID_ENCODING", e.to_string()))?
        .to_string();

    let size = query.size.unwrap_or(512);

    let png_bytes = did_to_png(&did, size).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes).into_response())
}

async fn get_palette(
    Path(did): Path<String>,
) -> std::result::Result<Json<PaletteResponse>, ApiError> {
    let did = urlencoding::decode(&did)
        .map_err(|e| ApiError::bad_request("INVALID_ENCODING", e.to_string()))?
        .to_string();

    // Validate DID format
    crate::ParsedDid::parse(&did)
        .map_err(|e| ApiError::bad_request("INVALID_DID", e.to_string()))?;

    let palette = GlyphPalette::from_did(&did);
    Ok(Json(PaletteResponse::from(&palette)))
}

async fn glyph_info(
    Json(req): Json<GlyphInfoRequest>,
) -> std::result::Result<Json<GlyphInfoResponse>, ApiError> {
    let parsed = crate::ParsedDid::parse(&req.did)
        .map_err(|e| ApiError::bad_request("INVALID_DID", e.to_string()))?;

    let payload = crate::encoding::payload::encode_did_payload(&parsed)
        .map_err(|e| ApiError::bad_request("ENCODING_FAILED", e.to_string()))?;

    let ecc_info = crate::encoding::reed_solomon::ecc_info(payload.len());

    let codeword = crate::encoding::reed_solomon::encode_with_ecc(&payload)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let grid = crate::encoding::grid::GlyphGrid::from_codeword(&codeword, parsed.kind)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(GlyphInfoResponse {
        did: req.did,
        kind: parsed.kind.as_str().to_string(),
        namespace: parsed.namespace,
        identifier: parsed.identifier,
        grid_size: crate::encoding::grid::GRID_SIZE,
        data_cells: grid.data_cell_count,
        payload_bytes: ecc_info.data_bytes,
        ecc_bytes: ecc_info.parity_bytes,
        total_codeword_bytes: ecc_info.total_bytes,
        error_correction_percent: ecc_info.correction_capacity_percent,
    }))
}
