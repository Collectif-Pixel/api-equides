use super::EtatPartage;
use super::equides::json;
use super::erreur::Erreur;
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Redirect, Response};
use http_body_util::BodyExt;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower::ServiceExt;
use tower_http::services::ServeFile;

pub const TELECHARGEMENTS_MAX: usize = 8;

#[derive(Serialize)]
struct Extraction {
    empreinte_source: String,
    ingere_le: String,
    octets: u64,
    #[serde(skip)]
    chemin: PathBuf,
}

pub struct Extractions {
    liste: Vec<Extraction>,
    pub empreinte: String,
    places: Arc<Semaphore>,
}

impl Extractions {
    pub fn lire(dossier: &std::path::Path) -> Result<Self> {
        let entrees = match std::fs::read_dir(dossier) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::depuis(Vec::new()));
            }
            r => r.with_context(|| format!("lecture de {}", dossier.display()))?,
        };
        let mut liste = Vec::new();
        for entree in entrees {
            let chemin = entree?.path();
            let Some(nom) = chemin
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".jsonl.zst"))
            else {
                continue;
            };
            let Some((date, empreinte)) = nom.split_once('_').filter(|(date, empreinte)| {
                crate::date::parse_iso(date).is_some()
                    && !empreinte.is_empty()
                    && empreinte
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            }) else {
                tracing::warn!(
                    fichier = %chemin.display(),
                    "ignoré : une extraction se nomme <AAAA-MM-JJ>_<empreinte_source>.jsonl.zst"
                );
                continue;
            };
            let octets = match chemin.metadata() {
                Ok(m) if m.is_file() => m.len(),
                _ => {
                    tracing::warn!(fichier = %chemin.display(), "ignoré : pas un fichier lisible");
                    continue;
                }
            };
            liste.push(Extraction {
                empreinte_source: empreinte.into(),
                ingere_le: date.into(),
                octets,
                chemin,
            });
        }
        liste.sort_by(|a, b| b.chemin.cmp(&a.chemin));
        let mut vues = HashSet::new();
        liste.retain(|e| {
            let premiere = vues.insert(e.empreinte_source.clone());
            if !premiere {
                tracing::warn!(
                    fichier = %e.chemin.display(),
                    "ignoré : une autre extraction porte déjà cette empreinte"
                );
            }
            premiere
        });
        Ok(Self::depuis(liste))
    }

    fn depuis(liste: Vec<Extraction>) -> Self {
        let mut h = blake3::Hasher::new();
        for e in &liste {
            h.update(e.chemin.as_os_str().as_encoded_bytes());
            h.update(&e.octets.to_le_bytes());
        }
        Self {
            liste,
            empreinte: h.finalize().to_hex().to_string(),
            places: Arc::new(Semaphore::new(TELECHARGEMENTS_MAX)),
        }
    }

    fn trouver(&self, empreinte: &str) -> Option<&Extraction> {
        self.liste.iter().find(|e| e.empreinte_source == empreinte)
    }

    pub fn publie(&self, empreinte: &str) -> bool {
        self.trouver(empreinte).is_some()
    }
}

#[derive(Serialize)]
struct Ligne<'a> {
    #[serde(flatten)]
    extraction: &'a Extraction,
    servie: bool,
    telechargement: String,
}

pub async fn lister(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    if etat.extractions.liste.is_empty() {
        return Err(Erreur::introuvable(
            "cette instance ne publie aucune extraction",
        ));
    }
    let donnees: Vec<Ligne> = etat
        .extractions
        .liste
        .iter()
        .map(|e| Ligne {
            extraction: e,
            servie: e.empreinte_source == etat.store.empreinte(),
            telechargement: format!("/v1/extractions/{}", e.empreinte_source),
        })
        .collect();
    json(&serde_json::json!({ "donnees": donnees }))
}

pub async fn derniere(State(etat): State<EtatPartage>) -> Result<Response, Erreur> {
    let empreinte = etat.store.empreinte();
    etat.extractions.trouver(empreinte).ok_or_else(|| {
        Erreur::introuvable(format!(
            "l'extraction servie, d'empreinte « {empreinte} », n'est pas encore publiée"
        ))
    })?;
    Ok(Redirect::temporary(&format!("/v1/extractions/{empreinte}")).into_response())
}

pub async fn par_empreinte(
    State(etat): State<EtatPartage>,
    Path(empreinte): Path<String>,
    req: Request,
) -> Result<Response, Erreur> {
    let e = etat.extractions.trouver(&empreinte).ok_or_else(|| {
        Erreur::introuvable(format!(
            "aucune extraction publiée ne porte l'empreinte « {empreinte} »"
        ))
    })?;
    let Ok(place) = etat.extractions.places.clone().try_acquire_owned() else {
        let mut reponse = Erreur::service_surcharge(format!(
            "{TELECHARGEMENTS_MAX} téléchargements sont déjà en cours sur cette instance"
        ))
        .into_response();
        reponse
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("30"));
        return Ok(reponse);
    };
    let Ok(reponse) = ServeFile::new(&e.chemin).oneshot(req).await;
    let (mut parts, corps) = reponse.into_response().into_parts();
    if parts.status.is_success() {
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/zstd"),
        );
        if let Ok(v) = HeaderValue::try_from(format!(
            "attachment; filename=\"chevaux-{}.jsonl.zst\"",
            e.ingere_le
        )) {
            parts.headers.insert(header::CONTENT_DISPOSITION, v);
        }
    }
    let corps = corps.map_frame(move |trame| {
        let _garde = &place;
        trame
    });
    Ok(Response::from_parts(parts, Body::new(corps)))
}
