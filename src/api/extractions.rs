use super::EtatPartage;
use super::equides::json;
use super::erreur::Erreur;
use anyhow::{Context, Result};
use axum::extract::{Path, Request, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Serialize;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[derive(Serialize)]
struct Extraction {
    empreinte_source: String,
    ingere_le: String,
    octets: u64,
    #[serde(skip)]
    chemin: PathBuf,
}

#[derive(Default)]
pub struct Extractions {
    liste: Vec<Extraction>,
    pub empreinte: String,
}

impl Extractions {
    pub fn lire(dossier: &std::path::Path) -> Result<Self> {
        let entrees = match std::fs::read_dir(dossier) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            r => r.with_context(|| format!("lecture de {}", dossier.display()))?,
        };
        let mut liste = Vec::new();
        for entree in entrees {
            let chemin = entree?.path();
            let Some((date, empreinte)) = chemin
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".jsonl.zst"))
                .and_then(|n| n.split_once('_'))
            else {
                continue;
            };
            liste.push(Extraction {
                empreinte_source: empreinte.into(),
                ingere_le: date.into(),
                octets: chemin.metadata()?.len(),
                chemin,
            });
        }
        liste.sort_by(|a, b| b.chemin.cmp(&a.chemin));
        let mut h = blake3::Hasher::new();
        for e in &liste {
            h.update(e.chemin.as_os_str().as_encoded_bytes());
            h.update(&e.octets.to_le_bytes());
        }
        Ok(Self {
            liste,
            empreinte: h.finalize().to_hex().to_string(),
        })
    }

    fn trouver(&self, empreinte: &str) -> Option<&Extraction> {
        self.liste.iter().find(|e| e.empreinte_source == empreinte)
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
    let Ok(mut reponse) = ServeFile::new(&e.chemin).oneshot(req).await;
    reponse.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zstd"),
    );
    Ok(reponse.into_response())
}
