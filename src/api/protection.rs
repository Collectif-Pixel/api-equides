use super::erreur::Erreur;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, Quota};
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct IpReelle {
    relais: Arc<Vec<IpAddr>>,
}

impl IpReelle {
    pub fn depuis_liste(liste: Option<&str>) -> Self {
        let relais = liste
            .unwrap_or("")
            .split(',')
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect::<Vec<_>>();
        if relais.is_empty() {
            tracing::warn!(
                "aucun relais approuvé : X-Forwarded-For sera ignoré et la limitation \
                 de débit s'appuiera sur l'adresse du pair"
            );
        } else {
            tracing::info!(
                relais = relais.len(),
                "relais approuvés pour X-Forwarded-For"
            );
        }
        Self {
            relais: Arc::new(relais),
        }
    }

    fn approuve(&self, ip: &IpAddr) -> bool {
        self.relais.contains(ip)
    }

    pub fn extraire<T>(&self, req: &axum::http::Request<T>) -> Option<IpAddr> {
        let pair = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())?;

        if !self.approuve(&pair) {
            return Some(pair);
        }

        let chaine = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let client = chaine
            .split(',')
            .rev()
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .find(|ip| !self.approuve(ip));

        Some(client.unwrap_or(pair))
    }
}

const OCTETS_PAR_UNITE: u32 = 16 * 1024;

mod octets {
    pub const RESUME: u32 = 139;
    pub const FICHE_COMPLETE: u32 = 795;
    pub const SUGGESTION: u32 = 135;
    pub const REFERENCE: u32 = 100;
    pub const TRANCHE: u32 = 41;
    pub const NOEUD_PEDIGREE: u32 = 108;
    pub const REFERENTIEL: u32 = super::super::referentiels::MODALITES_MAX as u32 * 40;
}

fn entier(query: Option<&str>, cle: &str) -> Option<usize> {
    form_urlencoded::parse(query.unwrap_or("").as_bytes())
        .find(|(k, _)| k == cle)
        .and_then(|(_, v)| v.trim().parse().ok())
}

fn contient(query: Option<&str>, cle: &str, valeur: &str) -> bool {
    form_urlencoded::parse(query.unwrap_or("").as_bytes())
        .any(|(k, v)| k == cle && v.trim() == valeur)
}

pub fn cout(chemin: &str, query: Option<&str>) -> u32 {
    let lignes = |defaut: usize, plafond: usize| -> u32 {
        entier(query, "limite").unwrap_or(defaut).clamp(1, plafond) as u32
    };

    let octets = match chemin {
        "/v1/equides" => {
            let par_ligne = if contient(query, "vue", "complet") {
                octets::FICHE_COMPLETE
            } else {
                octets::RESUME
            };
            lignes(crate::query::LIMITE_DEFAUT, crate::query::LIMITE_MAX) * par_ligne
        }
        "/v1/search" => {
            lignes(super::equides::SUGGESTIONS_DEFAUT, crate::query::LIMITE_MAX)
                * octets::SUGGESTION
        }
        "/v1/stats/repartition" => {
            lignes(
                super::stats::MODALITES_REPARTITION_DEFAUT,
                super::stats::MODALITES_REPARTITION_MAX,
            ) * octets::TRANCHE
        }
        c if c.ends_with("/descendance") => {
            lignes(super::equides::DESCENDANCE_DEFAUT, crate::query::LIMITE_MAX) * octets::REFERENCE
        }
        c if c.ends_with("/pedigree") => {
            let g = entier(query, "generations")
                .unwrap_or(super::equides::GENERATIONS_DEFAUT as usize)
                .min(crate::query::GENERATIONS_MAX as usize) as u32;
            (2u32.saturating_pow(g + 1) - 1).saturating_mul(octets::NOEUD_PEDIGREE)
        }
        c if c.starts_with("/v1/referentiels/") => octets::REFERENTIEL,
        _ => 0,
    };

    let plancher = u32::from(chemin == "/v1/stats/repartition");

    octets.div_ceil(OCTETS_PAR_UNITE).max(plancher).max(1)
}

pub const DEBIT_PAR_SECONDE: u32 = 50;

pub const RAFALE: u32 = 200;

const PURGE_TOUTES_LES: u64 = 8_192;

#[derive(Clone)]
pub struct Debit {
    limiteur: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    ip: IpReelle,
    vues: Arc<AtomicU64>,
}

impl Debit {
    pub fn nouveau(ip: IpReelle) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(DEBIT_PAR_SECONDE).expect("débit non nul"))
            .allow_burst(NonZeroU32::new(RAFALE).expect("rafale non nulle"));
        Self {
            limiteur: Arc::new(DefaultKeyedRateLimiter::keyed(quota)),
            ip,
            vues: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub async fn limiter_debit(State(d): State<Debit>, req: Request, next: Next) -> Response {
    if d.vues
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(PURGE_TOUTES_LES)
    {
        d.limiteur.retain_recent();
    }

    let Some(adresse) = d.ip.extraire(&req) else {
        return Erreur::requete_invalide("appelant non identifiable").into_response();
    };

    let cout = cout(req.uri().path(), req.uri().query()).min(RAFALE);
    let n = NonZeroU32::new(cout).expect("le coût vaut au moins une unité");

    match d.limiteur.check_key_n(&adresse, n) {
        Ok(Ok(())) => next.run(req).await,
        Ok(Err(pas_avant)) => {
            let attente = pas_avant.wait_time_from(DefaultClock::default().now());
            let secondes = attente.as_secs().max(1);
            let mut reponse = Erreur::trop_de_requetes(format!(
                "quota dépassé : {DEBIT_PAR_SECONDE} unités par seconde et par adresse, \
                 rafale de {RAFALE}. Cette requête en coûte {cout}. Réessayez dans {secondes} s."
            ))
            .into_response();
            if let Ok(v) = header::HeaderValue::from_str(&secondes.to_string()) {
                reponse.headers_mut().insert(header::RETRY_AFTER, v);
            }
            reponse
        }
        Err(_) => Erreur::requete_invalide(format!(
            "requête trop coûteuse : {cout} unités pour une rafale de {RAFALE}"
        ))
        .into_response(),
    }
}

pub fn concurrence_max() -> usize {
    std::thread::available_parallelism()
        .map_or(8, |n| n.get().saturating_mul(4))
        .clamp(8, 64)
}

static PLACES: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(concurrence_max()));

pub async fn limiter_concurrence(req: Request, next: Next) -> Response {
    let Ok(_place) = PLACES.try_acquire() else {
        let mut reponse = Erreur::service_surcharge(format!(
            "le service traite déjà {} requêtes simultanées",
            concurrence_max()
        ))
        .into_response();
        reponse
            .headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
        return reponse;
    };
    next.run(req).await
}

pub const PARAMS_MAX: usize = 64;

pub const QUERY_MAX: usize = 4096;

pub fn verifier_taille_requete(query: Option<&str>) -> Result<(), Erreur> {
    let Some(q) = query else {
        return Ok(());
    };
    if q.len() > QUERY_MAX {
        return Err(Erreur::parametre_invalide(
            "query",
            format!(
                "chaîne de requête de {} octets, {QUERY_MAX} au maximum",
                q.len()
            ),
            "combinez moins de valeurs, ou filtrez en plusieurs appels",
        ));
    }
    let nb = q.split('&').filter(|p| !p.is_empty()).count();
    if nb > PARAMS_MAX {
        return Err(Erreur::parametre_invalide(
            "query",
            format!("{nb} paramètres, {PARAMS_MAX} au maximum"),
            "chaque valeur supplémentaire coûte une résolution : \
             combinez moins de valeurs par appel",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;
    use axum::http::StatusCode;

    fn requete(pair: &str, xff: Option<&str>) -> HttpRequest<()> {
        let mut r = HttpRequest::builder().uri("/");
        if let Some(v) = xff {
            r = r.header("x-forwarded-for", v);
        }
        let mut req = r.body(()).unwrap();
        req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
            pair.parse::<IpAddr>().unwrap(),
            1234,
        )));
        req
    }

    #[test]
    fn un_client_direct_ne_choisit_pas_son_identite() {
        let e = IpReelle::depuis_liste(Some("10.0.0.1"));
        let k = e
            .extraire(&requete("203.0.113.9", Some("1.2.3.4")))
            .unwrap();
        assert_eq!(k.to_string(), "203.0.113.9");
    }

    #[test]
    fn derriere_un_relais_approuve_l_appelant_est_le_dernier_saut_inconnu() {
        let e = IpReelle::depuis_liste(Some("10.0.0.1, 10.0.0.2"));
        let k = e
            .extraire(&requete("10.0.0.1", Some("1.2.3.4, 203.0.113.9, 10.0.0.2")))
            .unwrap();
        assert_eq!(k.to_string(), "203.0.113.9");
    }

    #[test]
    fn sans_relais_approuve_l_en_tete_est_ignore() {
        let e = IpReelle::depuis_liste(None);
        let k = e
            .extraire(&requete("203.0.113.9", Some("1.2.3.4")))
            .unwrap();
        assert_eq!(k.to_string(), "203.0.113.9");
    }

    #[test]
    fn une_chaine_entierement_approuvee_retombe_sur_le_pair() {
        let e = IpReelle::depuis_liste(Some("10.0.0.1,10.0.0.2"));
        let k = e.extraire(&requete("10.0.0.1", Some("10.0.0.2"))).unwrap();
        assert_eq!(k.to_string(), "10.0.0.1");
    }

    #[test]
    fn sans_adresse_de_pair_l_extraction_echoue() {
        let e = IpReelle::depuis_liste(None);
        let req = HttpRequest::builder().uri("/").body(()).unwrap();
        assert!(e.extraire(&req).is_none());
    }

    #[test]
    fn les_requetes_demesurees_sont_refusees() {
        assert!(verifier_taille_requete(None).is_ok());
        assert!(verifier_taille_requete(Some("race=A&sexe=Femelle")).is_ok());

        let trop_long = format!("q={}", "a".repeat(QUERY_MAX));
        assert!(verifier_taille_requete(Some(&trop_long)).is_err());

        let trop_nombreux = (0..=PARAMS_MAX)
            .map(|i| format!("race=R{i}"))
            .collect::<Vec<_>>()
            .join("&");
        let err = verifier_taille_requete(Some(&trop_nombreux)).unwrap_err();
        assert_eq!(err.statut, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn le_plafond_de_parametres_laisse_passer_un_usage_normal() {
        let normal = "race=A&race=B&race=C&sexe=Femelle&annee_naissance=2015\
                      &annee_naissance=2016&robe=Bai&limite=100&offset=0&tri=-annee";
        assert!(verifier_taille_requete(Some(normal)).is_ok());
    }

    #[test]
    fn une_requete_ordinaire_coute_une_unite() {
        for (chemin, query) in [
            ("/healthz", None),
            ("/readyz", None),
            ("/v1/equides/Z4ogLhlkS2CeUdq0bZ0YFw", None),
            ("/v1/meta", None),
            ("/v1/stats", None),
            ("/openapi.json", None),
            ("/", None),
            ("/v1/equides", None),
            ("/v1/equides", Some("limite=100")),
            ("/v1/equides", Some("race=Trotteur%20Francais&limite=100")),
            ("/v1/search", Some("q=qab")),
            ("/v1/search", Some("q=qab&limite=100")),
            ("/v1/equides/X/pedigree", None),
            ("/v1/equides/X/pedigree", Some("generations=6")),
            ("/v1/equides/X/descendance", Some("limite=100")),
        ] {
            assert_eq!(cout(chemin, query), 1, "{chemin}?{query:?}");
        }
    }

    #[test]
    fn les_requetes_lourdes_coutent_davantage() {
        assert_eq!(cout("/v1/equides", Some("limite=100&vue=complet")), 5);
        assert_eq!(cout("/v1/equides/X/pedigree", Some("generations=8")), 4);
        assert_eq!(cout("/v1/referentiels/races", None), 5);
        assert_eq!(cout("/v1/stats/repartition", Some("limite=1")), 1);
        assert_eq!(cout("/v1/stats/repartition", Some("limite=1000")), 3);
    }

    #[test]
    fn le_cout_reste_borne_sur_des_parametres_hostiles() {
        for query in [
            "limite=999999999999",
            "limite=-1",
            "limite=abc",
            "limite=0",
            "generations=4294967295",
            "generations=999999999999999999999",
            "vue=complet&limite=18446744073709551615",
        ] {
            for chemin in [
                "/v1/equides",
                "/v1/search",
                "/v1/equides/X/pedigree",
                "/v1/equides/X/descendance",
                "/v1/stats/repartition",
                "/v1/referentiels/races",
            ] {
                let c = cout(chemin, Some(query));
                assert!(
                    (1..=RAFALE).contains(&c),
                    "coût hors bornes : {chemin}?{query} → {c}"
                );
            }
        }
    }

    #[test]
    fn le_cout_du_pedigree_croit_avec_la_profondeur() {
        let couts: Vec<u32> = (0..=crate::query::GENERATIONS_MAX)
            .map(|g| cout("/v1/equides/X/pedigree", Some(&format!("generations={g}"))))
            .collect();
        assert!(
            couts.windows(2).all(|w| w[1] >= w[0]),
            "coûts non croissants : {couts:?}"
        );
        assert!(
            couts.last() > couts.first(),
            "la profondeur maximale doit coûter plus que zéro génération"
        );
    }

    #[test]
    fn la_concurrence_suit_les_coeurs_disponibles() {
        let n = concurrence_max();
        assert!((8..=64).contains(&n), "concurrence hors bornes : {n}");
    }
}
