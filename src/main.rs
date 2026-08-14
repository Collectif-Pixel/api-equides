use anyhow::{Context, Result};
use axum::ServiceExt;
use axum::extract::Request;
use clap::Parser;
use equides_api::api::{Etat, router, router_admin};
use equides_api::metrics::Metrics;
use equides_api::snapshot::Snapshot;
use equides_api::store::Store;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

#[derive(Parser)]
#[command(
    name = "equides-api",
    version,
    about = "Sert l'API Équidés à partir d'une image produite par equides-ingest"
)]
struct Args {
    #[arg(long, env = "EQUIDES_IMAGE", default_value = "data/equides.bin")]
    image: PathBuf,

    #[arg(long, env = "EQUIDES_ADRESSE", default_value = "0.0.0.0:8081")]
    adresse: SocketAddr,

    #[arg(long, env = "EQUIDES_ADRESSE_ADMIN", default_value = "127.0.0.1:9091")]
    adresse_admin: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "equides_api=info,tower_http=warn".into()),
        )
        .init();

    let args = Args::parse();

    let debut = Instant::now();
    tracing::info!(image = %args.image.display(), "chargement de l'image");
    let snapshot = Snapshot::read(&args.image)?;
    let lignes = snapshot.meta.lignes;
    let source = snapshot.meta.source.clone();
    let charge = debut.elapsed();

    let debut_index = Instant::now();
    let store = Store::from_snapshot(snapshot);
    tracing::info!(
        lignes,
        source,
        lecture_ms = charge.as_millis(),
        indexation_ms = debut_index.elapsed().as_millis(),
        jetons_de_noms = store.noms_index.len(),
        "jeu de données prêt"
    );

    let etat = Arc::new(Etat {
        store,
        metrics: Metrics::new(),
    });

    let app = NormalizePathLayer::trim_trailing_slash().layer(router(etat.clone()));

    let listener = tokio::net::TcpListener::bind(args.adresse)
        .await
        .with_context(|| format!("écoute sur {}", args.adresse))?;
    let listener_admin = tokio::net::TcpListener::bind(args.adresse_admin)
        .await
        .with_context(|| format!("écoute d'administration sur {}", args.adresse_admin))?;
    tracing::info!(
        adresse = %args.adresse,
        adresse_admin = %args.adresse_admin,
        "serveur démarré"
    );

    let public = axum::serve(
        listener,
        ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app),
    )
    .with_graceful_shutdown(extinction());

    let admin = axum::serve(listener_admin, router_admin(etat).into_make_service())
        .with_graceful_shutdown(extinction());

    let (r_public, r_admin) = tokio::join!(public, admin);
    r_public.context("boucle du serveur public")?;
    r_admin.context("boucle du serveur d'administration")?;

    tracing::info!("arrêt propre terminé");
    Ok(())
}

async fn extinction() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installation du gestionnaire Ctrl-C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installation du gestionnaire SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("SIGINT reçu, extinction"),
        () = terminate => tracing::info!("SIGTERM reçu, extinction"),
    }
}
