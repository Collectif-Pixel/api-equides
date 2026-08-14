pub mod docs;
pub mod equides;
pub mod erreur;
pub mod meta;
pub mod params;
pub mod protection;
pub mod referentiels;
pub mod stats;

use crate::metrics::Metrics;
use crate::store::Store;
use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub struct Etat {
    pub store: Store,
    pub metrics: Metrics,
}

pub type EtatPartage = Arc<Etat>;

const CACHE_CONTROL: &str = "public, max-age=3600, stale-while-revalidate=86400";

pub const DELAI_MAX: Duration = Duration::from_secs(5);

pub fn router(etat: EtatPartage) -> Router {
    let debit = protection::Debit::nouveau(protection::IpReelle::depuis_liste(
        std::env::var("CC_REVERSE_PROXY_IPS").ok().as_deref(),
    ));

    Router::new()
        .route("/", get(meta::racine))
        .route("/v1/equides", get(equides::lister))
        .route("/v1/search", get(equides::rechercher))
        .route("/v1/equides/{id}", get(equides::par_id))
        .route("/v1/equides/{id}/pedigree", get(equides::pedigree))
        .route("/v1/equides/{id}/descendance", get(equides::descendance))
        .route("/v1/referentiels/{dimension}", get(referentiels::lister))
        .route("/v1/stats", get(stats::global))
        .route("/v1/stats/repartition", get(stats::repartition))
        .route("/v1/meta", get(meta::meta))
        .route("/openapi.json", get(meta::openapi))
        .route("/healthz", get(meta::sante))
        .route("/readyz", get(meta::sante))
        .layer(
            ServiceBuilder::new()
                .layer(CatchPanicLayer::new())
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn_with_state(etat.clone(), mesurer))
                .layer(SetResponseHeaderLayer::overriding(
                    HeaderName::from_static("x-content-type-options"),
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("no-referrer"),
                ))
                .layer(axum::middleware::from_fn(protection::limiter_concurrence))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::SERVICE_UNAVAILABLE,
                    DELAI_MAX,
                ))
                .layer(axum::middleware::from_fn_with_state(
                    debit,
                    protection::limiter_debit,
                ))
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([axum::http::Method::GET, axum::http::Method::HEAD])
                        .allow_headers([header::IF_NONE_MATCH])
                        .expose_headers([header::ETAG]),
                )
                .layer(axum::middleware::from_fn_with_state(etat.clone(), cache))
                .layer(CompressionLayer::new()),
        )
        .fallback(meta::route_inconnue)
        .with_state(etat)
}

pub fn router_admin(etat: EtatPartage) -> Router {
    Router::new()
        .route("/metrics", get(meta::metriques))
        .route("/healthz", get(meta::sante))
        .route("/readyz", get(meta::sante))
        .layer(CatchPanicLayer::new())
        .with_state(etat)
}

async fn mesurer(State(etat): State<EtatPartage>, req: Request, next: Next) -> Response {
    let debut = Instant::now();
    let reponse = next.run(req).await;
    etat.metrics
        .observer(reponse.status().as_u16(), debut.elapsed().as_secs_f64());
    reponse
}

async fn cache(State(etat): State<EtatPartage>, req: Request, next: Next) -> Response {
    let mut h = blake3::Hasher::new();
    h.update(etat.store.empreinte().as_bytes());
    h.update(req.uri().path().as_bytes());
    h.update(b"?");
    h.update(req.uri().query().unwrap_or("").as_bytes());
    h.update(
        req.headers()
            .get(header::ACCEPT_ENCODING)
            .map_or(b"".as_slice(), |v| v.as_bytes()),
    );
    let etag = format!("\"{}\"", &h.finalize().to_hex()[..24]);

    let deja_a_jour = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|e| e.trim() == etag));

    if deja_a_jour {
        let mut reponse = Response::new(axum::body::Body::empty());
        *reponse.status_mut() = StatusCode::NOT_MODIFIED;
        ajouter_entetes_cache(&mut reponse, &etag);
        return reponse;
    }

    let mut reponse = next.run(req).await;
    if reponse.status().is_success() {
        ajouter_entetes_cache(&mut reponse, &etag);
    }
    reponse
}

fn ajouter_entetes_cache(reponse: &mut Response, etag: &str) {
    let entetes = reponse.headers_mut();
    if let Ok(v) = HeaderValue::from_str(etag) {
        entetes.insert(header::ETAG, v);
    }
    entetes.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL),
    );
}
