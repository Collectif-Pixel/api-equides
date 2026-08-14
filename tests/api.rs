use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use equides_api::api::{Etat, router, router_admin};
use equides_api::ids;
use equides_api::metrics::Metrics;
use equides_api::snapshot::{
    ANNEE_ABSENTE, Anomalies, Arene, COEFFICIENT_ABSENT, DatasetMeta, FORMAT_VERSION,
    SANS_MODALITE, SANS_PARENT, Snapshot,
};
use equides_api::store::Store;
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

type LigneTest = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    i32,
    &'static str,
    &'static str,
);

const LIGNES: &[LigneTest] = &[
    (
        "AAAAAAAAAAAAAAAAAAAAAA",
        "GRAND PERE",
        "Trotteur Francais",
        "Male",
        "Bai",
        1990,
        "",
        "",
    ),
    (
        "BBBBBBBBBBBBBBBBBBBBBA",
        "GRAND MERE",
        "Trotteur Francais",
        "Femelle",
        "Alezan",
        1992,
        "",
        "",
    ),
    (
        "CCCCCCCCCCCCCCCCCCCCCA",
        "PERE",
        "Trotteur Francais",
        "Male",
        "Bai",
        2005,
        "AAAAAAAAAAAAAAAAAAAAAA",
        "BBBBBBBBBBBBBBBBBBBBBA",
    ),
    (
        "DDDDDDDDDDDDDDDDDDDDDA",
        "MERE",
        "Pur Sang",
        "Femelle",
        "Gris",
        2006,
        "",
        "",
    ),
    (
        "EEEEEEEEEEEEEEEEEEEEEA",
        "ELDORÀDO",
        "Trotteur Francais",
        "Femelle",
        "Bai",
        2015,
        "CCCCCCCCCCCCCCCCCCCCCA",
        "DDDDDDDDDDDDDDDDDDDDDA",
    ),
    (
        "FFFFFFFFFFFFFFFFFFFFFA",
        "ORPHELIN",
        "Pur Sang",
        "Hongre",
        "",
        2020,
        "ZZZZZZZZZZZZZZZZZZZZZA",
        "",
    ),
    (
        "GGGGGGGGGGGGGGGGGGGGGA",
        "SANS DATE",
        "Pur Sang",
        "Indeter",
        "Noir",
        0,
        "",
        "",
    ),
];

fn snapshot_test() -> Snapshot {
    let mut dicts: [Vec<String>; 3] = Default::default();
    let intern = |d: &mut Vec<String>, v: &str| -> u16 {
        if let Some(i) = d.iter().position(|x| x == v) {
            i as u16
        } else {
            d.push(v.to_string());
            (d.len() - 1) as u16
        }
    };

    let n = LIGNES.len();
    let mut ids_plats = Vec::new();
    let mut noms = Arene::nouvelle();
    let mut slugs = Arene::nouvelle();
    let mut filiations = Arene::nouvelle();
    let (mut race, mut robe, mut sexe, mut annee) = (vec![], vec![], vec![], vec![]);
    let mut index: HashMap<&str, u32> = HashMap::new();

    for (i, l) in LIGNES.iter().enumerate() {
        index.insert(l.0, i as u32);
        ids_plats.extend_from_slice(&ids::decode(l.0).expect("identifiant de test valide"));
        noms.pousser(l.1).unwrap();
        slugs
            .pousser(&l.1.to_lowercase().replace(' ', "-"))
            .unwrap();
        filiations
            .pousser(if l.6.is_empty() {
                ""
            } else {
                "Par PERE et MERE"
            })
            .unwrap();
        race.push(intern(&mut dicts[0], l.2));
        sexe.push(intern(&mut dicts[2], l.3) as u8);
        robe.push(if l.4.is_empty() {
            SANS_MODALITE
        } else {
            intern(&mut dicts[1], l.4)
        });
        annee.push(if l.5 == 0 { ANNEE_ABSENTE } else { l.5 as i16 });
    }
    let lien = |cible: &str| -> u32 { index.get(cible).copied().unwrap_or(SANS_PARENT) };

    let mut perf_bornes = vec![0u32];
    let mut perf_discipline = Vec::new();
    let mut perf_textes = Arene::nouvelle();
    let mut ind_bornes = vec![0u32];
    let mut ind_code = Vec::new();
    let mut ind_valeurs = Arene::nouvelle();
    for i in 0..n {
        if i == 4 {
            perf_discipline.push(0);
            perf_textes.pousser("BTR +59 (0.35)").unwrap();
            ind_code.push(0);
            ind_valeurs.pousser("+59").unwrap();
            ind_bornes.push(ind_code.len() as u32);
        }
        perf_bornes.push(perf_discipline.len() as u32);
    }
    let nb_ind = ind_code.len();
    let [dict_races, dict_robes, dict_sexes] = dicts;

    Snapshot {
        format_version: FORMAT_VERSION,
        meta: DatasetMeta {
            producteur: "IFCE".into(),
            source: "Jeu synthétique de test".into(),
            licence: "Licence Ouverte / réutilisation d'informations publiques (CRPA art. L321-1)"
                .into(),
            url_modele: "https://infochevaux.ifce.fr/fr/{slug}-{id}/infos-generales".into(),
            ingere_le: "2026-01-01".into(),
            empreinte_source: "empreinte-de-test".into(),
            lignes: n as u32,
            anomalies: Anomalies {
                sans_filiation: 4,
                parents_pendants: 1,
                annee_naissance_absente: 1,
                annee_naissance_min: Some(1990),
                annee_naissance_max: Some(2020),
                ..Default::default()
            },
        },
        dict_races,
        dict_robes,
        dict_sexes,
        dict_races_lien: vec!["TF".into(), "PS".into()],
        dict_pays_lien: vec!["IRLANDE".into()],
        dict_disciplines: vec!["TROT COURSE".into()],
        dict_codes_indice: vec!["BTR".into()],
        dict_appreciations: vec![],
        ids: ids_plats,
        noms,
        slugs,
        filiations,
        race,
        robe,
        sexe,
        annee,
        pere: LIGNES.iter().map(|l| lien(l.6)).collect(),
        mere: LIGNES.iter().map(|l| lien(l.7)).collect(),
        pere_de_mere: vec![SANS_PARENT; n],
        lien_race: (0..n * 3)
            .map(|i| {
                let l = &LIGNES[i / 3];
                let declare =
                    matches!(i % 3, 0 if !l.6.is_empty()) || matches!(i % 3, 1 if !l.7.is_empty());
                if declare { 0 } else { SANS_MODALITE }
            })
            .collect(),
        lien_pays: (0..n * 3)
            .map(|i| if i == 4 * 3 + 1 { 0 } else { SANS_MODALITE })
            .collect(),
        lien_alias: vec![],
        liens_rompus: vec![(5, 0, "PERE INTROUVABLE".to_string())],
        perf_bornes,
        perf_discipline,
        perf_textes,
        ind_bornes,
        ind_code,
        ind_valeurs,
        ind_coefficient: vec![COEFFICIENT_ABSENT; nb_ind],
        ind_annee: vec![ANNEE_ABSENTE; nb_ind],
        ind_appreciation: vec![SANS_MODALITE; nb_ind],
    }
}

fn etat() -> Arc<Etat> {
    Arc::new(Etat {
        store: Store::from_snapshot(snapshot_test()),
        metrics: Metrics::new(),
    })
}

fn app() -> axum::Router {
    router(etat())
}

fn app_admin() -> axum::Router {
    router_admin(etat())
}

async fn get(uri: &str) -> (StatusCode, header::HeaderMap, String) {
    get_avec(uri, Vec::new()).await
}

async fn get_avec(
    uri: &str,
    entetes: Vec<(&str, &str)>,
) -> (StatusCode, header::HeaderMap, String) {
    let mut req = Request::builder().uri(uri).method("GET");
    for (k, v) in entetes {
        req = req.header(k, v);
    }
    let mut req = req.body(Body::empty()).unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [203, 0, 113, 7],
            1234,
        ))));
    let reponse = app().oneshot(req).await.expect("réponse du routeur");
    let statut = reponse.status();
    let entetes = reponse.headers().clone();
    let octets = axum::body::to_bytes(reponse.into_body(), 64 * 1024 * 1024)
        .await
        .expect("corps de réponse");
    (
        statut,
        entetes,
        String::from_utf8_lossy(&octets).into_owned(),
    )
}

async fn get_sur(app: &axum::Router, uri: &str) -> (StatusCode, header::HeaderMap) {
    let mut req = Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [203, 0, 113, 7],
            1234,
        ))));
    let reponse = app.clone().oneshot(req).await.expect("réponse du routeur");
    (reponse.status(), reponse.headers().clone())
}

async fn json(uri: &str) -> serde_json::Value {
    let (statut, _, corps) = get(uri).await;
    assert_eq!(statut, StatusCode::OK, "GET {uri} → {statut} : {corps}");
    serde_json::from_str(&corps).expect("corps JSON valide")
}

#[tokio::test]
async fn recherche_sans_filtre_renvoie_tout() {
    let v = json("/v1/equides").await;
    assert_eq!(v["pagination"]["total"], 7);
    assert_eq!(v["donnees"].as_array().unwrap().len(), 7);
}

#[tokio::test]
async fn les_listes_servent_un_resume_pas_la_fiche_entiere() {
    let e = &json("/v1/equides?nom=eldorado").await["donnees"][0];
    assert_eq!(e["nom"], "ELDORÀDO");
    assert_eq!(e["id"], "EEEEEEEEEEEEEEEEEEEEEA");
    for lourd in [
        "pere",
        "mere",
        "performances",
        "filiation_texte",
        "liens",
        "url",
        "href",
    ] {
        assert!(
            e.get(lourd).is_none(),
            "le résumé ne doit pas porter `{lourd}`"
        );
    }
}

#[tokio::test]
async fn la_vue_complete_reste_accessible_sur_demande() {
    let e = &json("/v1/equides?nom=eldorado&vue=complet").await["donnees"][0];
    assert_eq!(e["pere"]["nom"], "PERE");
    assert!(e["performances"].is_array());

    assert_eq!(
        get("/v1/equides?vue=detaille").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn le_resume_est_nettement_plus_leger_que_la_fiche() {
    let (_, _, resume) = get("/v1/equides").await;
    let (_, _, complet) = get("/v1/equides?vue=complet").await;
    assert!(
        resume.len() * 2 < complet.len(),
        "le résumé ({} o) devrait peser moins de la moitié de la vue complète ({} o)",
        resume.len(),
        complet.len()
    );
}

#[tokio::test]
async fn filtres_ou_dans_une_dimension_et_entre_dimensions() {
    assert_eq!(
        json("/v1/equides?race=Trotteur%20Francais&race=Pur%20Sang").await["pagination"]["total"],
        7
    );
    assert_eq!(
        json("/v1/equides?race=Trotteur%20Francais&sexe=Femelle").await["pagination"]["total"],
        2
    );
    assert_eq!(
        json("/v1/equides?sexe=Male,Hongre").await["pagination"]["total"],
        3
    );
}

#[tokio::test]
async fn filtres_insensibles_casse_et_accents() {
    assert_eq!(
        json("/v1/equides?race=trotteur+francais").await["pagination"]["total"],
        4
    );
    assert_eq!(
        json("/v1/equides?nom=eldorado").await["pagination"]["total"],
        1
    );
    assert_eq!(
        json("/v1/equides?nom=ELDORÀDO").await["pagination"]["total"],
        1
    );
}

#[tokio::test]
async fn intervalle_de_millesimes() {
    assert_eq!(
        json("/v1/equides?annee_min=2005&annee_max=2015").await["pagination"]["total"],
        3
    );
    assert_eq!(
        json("/v1/equides?annee_min=1").await["pagination"]["total"],
        6
    );
}

#[tokio::test]
async fn filtre_sur_les_performances() {
    assert_eq!(
        json("/v1/equides?avec_performances=true").await["pagination"]["total"],
        1
    );
    assert_eq!(
        json("/v1/equides?discipline=TROT%20COURSE").await["pagination"]["total"],
        1
    );
}

#[tokio::test]
async fn valeur_de_filtre_inconnue_donne_400_avec_indice() {
    let (statut, _, corps) = get("/v1/equides?race=Licorne").await;
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    let v: serde_json::Value = serde_json::from_str(&corps).unwrap();
    assert_eq!(v["parametre"], "race");
    assert!(
        v["indice"]
            .as_str()
            .unwrap()
            .contains("/v1/referentiels/races")
    );
}

#[tokio::test]
async fn parametre_inconnu_rejete_avec_suggestion() {
    let (statut, _, corps) = get("/v1/equides?rase=Pur%20Sang").await;
    assert_eq!(
        statut,
        StatusCode::BAD_REQUEST,
        "une faute de frappe ne doit pas renvoyer silencieusement tout le fichier"
    );
    let v: serde_json::Value = serde_json::from_str(&corps).unwrap();
    assert!(v["indice"].as_str().unwrap().contains("`race`"));
}

#[tokio::test]
async fn bornes_de_pagination_appliquees() {
    assert_eq!(
        get("/v1/equides?limite=201").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get("/v1/equides?offset=10001").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(get("/v1/equides?tri=robe").await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tri_place_les_millesimes_absents_en_dernier() {
    for tri in ["annee", "-annee"] {
        let v = json(&format!("/v1/equides?tri={tri}")).await;
        let dernier = v["donnees"].as_array().unwrap().last().unwrap();
        assert!(
            dernier["annee_naissance"].is_null(),
            "tri={tri} : la fiche sans millésime doit rester en fin de liste"
        );
    }
}

#[tokio::test]
async fn consultation_par_identifiant_officiel() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    assert_eq!(e["nom"], "ELDORÀDO");
    assert_eq!(e["id"], "EEEEEEEEEEEEEEEEEEEEEA");
    assert_eq!(e["race"], "Trotteur Francais");
    assert_eq!(e["annee_naissance"], 2015);
}

#[tokio::test]
async fn url_de_la_fiche_reconstruite() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    let url = e["url"].as_str().unwrap();
    assert!(url.starts_with("https://infochevaux.ifce.fr/fr/"));
    assert!(url.ends_with("/infos-generales"));
    assert!(url.contains("EEEEEEEEEEEEEEEEEEEEEA"));
}

#[tokio::test]
async fn identifiant_inconnu_ou_mal_forme_donne_404() {
    assert_eq!(
        get("/v1/equides/ZZZZZZZZZZZZZZZZZZZZZA").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get("/v1/equides/pas-un-identifiant").await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn la_fiche_expose_ses_parents() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    assert_eq!(e["pere"]["nom"], "PERE");
    assert_eq!(e["mere"]["nom"], "MERE");
    assert_eq!(e["pere"]["id"], "CCCCCCCCCCCCCCCCCCCCCA");
    assert_eq!(e["nombre_de_descendants"], 0);
    assert!(e.get("liens").is_none());
    assert!(e["pere"].get("href").is_none());
}

#[tokio::test]
async fn un_lien_rompu_reste_expose_sans_href() {
    let e = json("/v1/equides/FFFFFFFFFFFFFFFFFFFFFA").await;
    let pere = &e["pere"];
    assert_eq!(pere["nom"], "PERE INTROUVABLE");
    assert_eq!(pere["fiche_disponible"], false);
    assert!(pere.get("id").is_none());
}

#[tokio::test]
async fn la_race_du_parent_vient_du_lien_pas_de_sa_fiche() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    assert_eq!(e["pere"]["race"], "TF");
    assert_eq!(
        json("/v1/equides/CCCCCCCCCCCCCCCCCCCCCA").await["race"],
        "Trotteur Francais"
    );
}

#[tokio::test]
async fn les_attributs_du_lien_sont_exposes() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    assert_eq!(e["mere"]["pays"], "IRLANDE");
    assert_eq!(e["mere"]["fiche_disponible"], true);
    assert!(e["mere"]["id"].is_string());
    assert!(e["pere"].get("pays").is_none());
}

#[tokio::test]
async fn filtre_par_code_d_indice() {
    assert_eq!(
        json("/v1/equides?indice=BTR").await["pagination"]["total"],
        1
    );
    let (statut, _, corps) = get("/v1/equides?indice=XXX").await;
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert!(corps.contains("codes_indice"));
}

#[tokio::test]
async fn referentiels_des_codes_d_indice_et_races_de_lien() {
    let codes = json("/v1/referentiels/codes_indice").await;
    assert_eq!(codes["nombre_de_modalites"], 1);
    assert_eq!(codes["donnees"][0]["valeur"], "BTR");

    let races_lien = json("/v1/referentiels/races_lien").await;
    let valeurs: Vec<&str> = races_lien["donnees"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["valeur"].as_str().unwrap())
        .collect();
    assert!(valeurs.contains(&"TF"));
}

#[tokio::test]
async fn les_champs_absents_sont_omis() {
    let e = json("/v1/equides/GGGGGGGGGGGGGGGGGGGGGA").await;
    assert!(e.get("annee_naissance").is_none());
    assert!(e.get("naisseur").is_none());
    let orphelin = json("/v1/equides/FFFFFFFFFFFFFFFFFFFFFA").await;
    assert!(orphelin.get("robe").is_none());
}

#[tokio::test]
async fn les_performances_sont_exposees() {
    let e = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    let perfs = e["performances"].as_array().unwrap();
    assert_eq!(perfs.len(), 1);
    assert_eq!(perfs[0]["discipline"], "TROT COURSE");
    assert_eq!(perfs[0]["indices"][0]["code"], "BTR");
    assert_eq!(perfs[0]["indices"][0]["valeur"], "+59");
    assert!(
        json("/v1/equides/AAAAAAAAAAAAAAAAAAAAAA")
            .await
            .get("performances")
            .is_none()
    );
}

#[tokio::test]
async fn pedigree_remonte_les_generations() {
    let p = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/pedigree?generations=2").await;
    assert_eq!(p["generations"], 2);
    assert_eq!(p["arbre"]["nom"], "ELDORÀDO");
    assert_eq!(p["arbre"]["generation"], 0);
    assert_eq!(p["arbre"]["pere"]["nom"], "PERE");
    assert_eq!(p["arbre"]["pere"]["generation"], 1);
    assert_eq!(p["arbre"]["pere"]["pere"]["nom"], "GRAND PERE");
    assert_eq!(p["arbre"]["pere"]["mere"]["nom"], "GRAND MERE");
    assert!(p["arbre"]["mere"].get("pere").is_none());
}

#[tokio::test]
async fn pedigree_distingue_les_noeuds_connus_des_noeuds_theoriques() {
    let p = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/pedigree?generations=2").await;
    assert_eq!(p["noeuds_theoriques"], 7);
    assert_eq!(p["noeuds"], 5);
}

#[tokio::test]
async fn pedigree_borne_sa_profondeur() {
    let (statut, _, corps) =
        get("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/pedigree?generations=50").await;
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert!(corps.contains("generations"));

    for enorme in ["4294967296", "4294967306", "18446744073709551615"] {
        let (statut, _, _) = get(&format!(
            "/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/pedigree?generations={enorme}"
        ))
        .await;
        assert_eq!(
            statut,
            StatusCode::BAD_REQUEST,
            "generations={enorme} doit être refusé, non tronqué"
        );
    }

    let p = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/pedigree?generations=0").await;
    assert_eq!(p["noeuds"], 1);
}

#[tokio::test]
async fn descendance_inverse_la_filiation() {
    let d = json("/v1/equides/CCCCCCCCCCCCCCCCCCCCCA/descendance").await;
    assert_eq!(d["parent"]["nom"], "PERE");
    assert_eq!(d["total"], 1);
    assert_eq!(d["donnees"][0]["nom"], "ELDORÀDO");

    assert_eq!(
        json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA/descendance").await["total"],
        0
    );
}

#[tokio::test]
async fn descendance_et_pedigree_sur_identifiant_inconnu() {
    assert_eq!(
        get("/v1/equides/ZZZZZZZZZZZZZZZZZZZZZA/pedigree").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get("/v1/equides/ZZZZZZZZZZZZZZZZZZZZZA/descendance")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn referentiels() {
    let v = json("/v1/referentiels/races").await;
    assert_eq!(v["nombre_de_modalites"], 2);
    assert_eq!(v["donnees"][0]["nombre"], 4);

    let sexes = json("/v1/referentiels/sexes").await;
    let valeurs: Vec<&str> = sexes["donnees"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["valeur"].as_str().unwrap())
        .collect();
    assert!(valeurs.contains(&"Indeter"));

    assert_eq!(
        json("/v1/referentiels/disciplines").await["nombre_de_modalites"],
        1
    );
    assert_eq!(
        get("/v1/referentiels/couleurs").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn statistiques_globales() {
    let v = json("/v1/stats").await;
    assert_eq!(v["total"], 7);
    assert_eq!(v["avec_performances"], 1);
    assert_eq!(v["sans_performances"], 6);
    assert!(v["avertissement"].as_str().unwrap().contains("concouru"));
}

#[tokio::test]
async fn repartition_respecte_les_filtres() {
    let v = json("/v1/stats/repartition?dimension=robe&race=Trotteur%20Francais").await;
    assert_eq!(v["effectif_filtre"], 4);

    let annees = json("/v1/stats/repartition?dimension=annee_naissance").await;
    let valeurs: Vec<&str> = annees["donnees"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["valeur"].as_str().unwrap())
        .collect();
    assert_eq!(
        valeurs,
        vec!["1990", "1992", "2005", "2006", "2015", "2020"]
    );

    assert_eq!(
        get("/v1/stats/repartition?dimension=taille").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn les_exports_ont_ete_retires() {
    for route in ["/v1/export.csv", "/v1/export.ndjson"] {
        assert_eq!(get(route).await.0, StatusCode::NOT_FOUND, "{route}");
    }
}

#[tokio::test]
async fn cache_etag_et_revalidation() {
    let (statut, entetes, _) = get("/v1/equides").await;
    assert_eq!(statut, StatusCode::OK);
    let etag = entetes[header::ETAG].to_str().unwrap().to_string();
    assert!(
        entetes[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("max-age=3600"),
        "les réponses doivent être cacheables : c'est le principal levier de tenue en charge"
    );

    let (statut, _, corps) = get_avec("/v1/equides", vec![("if-none-match", &etag)]).await;
    assert_eq!(statut, StatusCode::NOT_MODIFIED);
    assert!(corps.is_empty(), "un 304 ne doit pas porter de corps");
}

#[tokio::test]
async fn etag_different_selon_la_requete() {
    let (_, a, _) = get("/v1/equides?race=Pur%20Sang").await;
    let (_, b, _) = get("/v1/equides?race=Trotteur%20Francais").await;
    assert_ne!(a[header::ETAG], b[header::ETAG]);
}

#[tokio::test]
async fn metadonnees_exposent_provenance_et_limites() {
    let v = json("/v1/meta").await;
    assert_eq!(v["jeu_de_donnees"]["lignes"], 7);
    assert_eq!(v["jeu_de_donnees"]["anomalies"]["parents_pendants"], 1);
    assert!(
        v["jeu_de_donnees"]["url_modele"]
            .as_str()
            .unwrap()
            .contains("{slug}")
    );
    let remarques = serde_json::to_string(&v["remarques"]).unwrap();
    assert!(remarques.contains("L321-1") || remarques.contains("Licence Ouverte"));
    assert!(remarques.contains("aucune donnée à caractère personnel"));
    assert!(remarques.contains("non affilié"));
    assert!(remarques.contains("liens rompus"));
    assert!(remarques.contains("11 747"));
}

#[tokio::test]
async fn les_sondes_restent_sur_le_port_public() {
    for route in ["/healthz", "/readyz"] {
        let (statut, _, corps) = get(route).await;
        assert_eq!(statut, StatusCode::OK, "{route}");
        assert!(corps.contains("\"statut\":\"ok\""));
    }
}

#[tokio::test]
async fn la_telemetrie_n_est_pas_exposee_publiquement() {
    assert_eq!(get("/metrics").await.0, StatusCode::NOT_FOUND);

    let reponse = app_admin()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps = axum::body::to_bytes(reponse.into_body(), 1 << 20)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&corps).contains("equides_lignes 7"));
}

#[tokio::test]
async fn aucune_donnee_personnelle_n_est_exposee() {
    let fiche = json("/v1/equides/EEEEEEEEEEEEEEEEEEEEEA").await;
    assert!(
        fiche.get("naisseur").is_none(),
        "la fiche ne doit pas porter de naisseur"
    );

    for route in [
        "/v1/equides?limite=100",
        "/v1/equides?limite=100&vue=complet",
    ] {
        let (_, _, corps) = get(route).await;
        assert!(!corps.contains("naisseur"), "{route}");
    }

    assert_eq!(
        get("/v1/referentiels/naisseurs").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get("/v1/equides?naisseur=Untel").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get("/v1/stats/repartition?dimension=naisseur").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn les_erreurs_suivent_la_rfc_9457() {
    let (statut, entetes, corps) = get("/v1/equides?race=Licorne").await;
    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert!(
        entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/problem+json"),
        "le type de média normalisé est ce qui rend l'erreur lisible par un client générique"
    );
    let v: serde_json::Value = serde_json::from_str(&corps).unwrap();
    for champ in ["type", "title", "status", "detail"] {
        assert!(v.get(champ).is_some(), "champ RFC 9457 manquant : {champ}");
    }
    assert_eq!(v["status"], 400);
    assert_eq!(v["parametre"], "race");
}

#[tokio::test]
async fn les_entetes_de_securite_sont_poses() {
    let (_, entetes, _) = get("/v1/equides").await;
    assert_eq!(entetes["x-content-type-options"], "nosniff");
    assert_eq!(entetes[header::REFERRER_POLICY], "no-referrer");
}

#[tokio::test]
async fn la_racine_sert_le_document_de_decouverte() {
    let v = json("/").await;

    assert_eq!(v["endpoints"]["autocompletion"], "/v1/search?q=");
    assert_eq!(v["gabarits_url"]["fiche"], "/v1/equides/{id}");
    assert!(v["licence_donnees"].is_string());
    assert_eq!(v["documentation"], "https://docs.api-equides.org");
}

#[tokio::test]
async fn document_openapi_servi() {
    let v = json("/openapi.json").await;
    assert_eq!(v["openapi"], "3.1.0");
    for chemin in [
        "/v1/equides",
        "/v1/equides/{id}",
        "/v1/equides/{id}/pedigree",
        "/v1/equides/{id}/descendance",
        "/v1/search",
    ] {
        assert!(v["paths"][chemin].is_object(), "chemin absent : {chemin}");
    }
    assert!(v["components"]["schemas"]["Noeud"].is_object());
}

#[tokio::test]
async fn la_racine_sert_la_decouverte_quel_que_soit_l_accept() {
    let (statut, entetes, _) = get_avec("/", vec![("accept", "text/html")]).await;
    assert_eq!(statut, StatusCode::OK);
    assert!(
        entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("application/json"),
        "la racine ne sert plus de page HTML"
    );

    let (statut, entetes, corps) = get_avec("/", vec![("accept", "application/json")]).await;
    assert_eq!(statut, StatusCode::OK);
    assert!(
        entetes[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("application/json")
    );
    let v: serde_json::Value = serde_json::from_str(&corps).unwrap();
    assert_eq!(v["lignes"], 7);
    assert!(v["affiliation"].as_str().unwrap().contains("non affilié"));
    assert_eq!(v["gabarits_url"]["fiche"], "/v1/equides/{id}");
    assert!(
        !v["endpoints"]["referentiels"]
            .as_str()
            .unwrap()
            .contains("naisseurs")
    );

    assert_eq!(get("/docs").await.0, StatusCode::NOT_FOUND);

    let (statut, _, corps) = get("/v1/chevaux").await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert!(corps.contains("introuvable"));
}

fn gabarit(chemin: &str) -> String {
    let sans_query = chemin.split('?').next().unwrap_or(chemin);
    let mut out = String::new();
    let mut dans_accolade = false;
    for c in sans_query.chars() {
        match c {
            '{' => {
                dans_accolade = true;
                out.push_str("{p}");
            }
            '}' => dans_accolade = false,
            _ if dans_accolade => {}
            _ => out.push(c),
        }
    }
    out
}

#[tokio::test]
async fn la_decouverte_n_annonce_que_des_routes_servies() {
    let (_, _, corps) = get_avec("/", vec![("accept", "application/json")]).await;
    let decouverte: serde_json::Value = serde_json::from_str(&corps).unwrap();
    let doc = json("/openapi.json").await;

    let decrits: Vec<String> = doc["paths"]
        .as_object()
        .unwrap()
        .keys()
        .map(|c| gabarit(c))
        .collect();

    let endpoints = decouverte["endpoints"].as_object().unwrap();
    assert!(!endpoints.is_empty());
    for (usage, chemin) in endpoints {
        let annonce = gabarit(chemin.as_str().unwrap());
        assert!(
            decrits.contains(&annonce),
            "la découverte annonce « {usage} » → {annonce}, que le service ne décrit pas"
        );
    }

    for (usage, chemin) in decouverte["gabarits_url"].as_object().unwrap() {
        let valeur = chemin.as_str().unwrap();
        if valeur.starts_with("http") {
            continue;
        }
        assert!(
            decrits.contains(&gabarit(valeur)),
            "gabarit « {usage} » vers une route inexistante : {valeur}"
        );
    }
}

#[tokio::test]
async fn le_type_d_une_erreur_pointe_vers_une_ancre_reelle() {
    let guide = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/src/content/docs/guides/erreurs.md"
    ))
    .expect("le guide des erreurs est versionné avec le service");

    for route in [
        "/v1/equides?rase=x",
        "/v1/equides/inexistant",
        "/v1/referentiels/inconnu",
    ] {
        let (_, _, corps) = get(route).await;
        let v: serde_json::Value = serde_json::from_str(&corps).unwrap();
        let type_ = v["type"].as_str().unwrap();
        assert!(
            type_.starts_with("https://docs.api-equides.org/guides/erreurs/#"),
            "{route} renvoie un type qui ne mène nulle part : {type_}"
        );
        let ancre = type_.rsplit_once('#').unwrap().1;
        assert!(
            guide.contains(&format!("id=\"{ancre}\"")),
            "{route} renvoie type={type_}, ancre absente du guide publié"
        );
    }
}

#[tokio::test]
async fn la_note_de_pagination_ne_renvoie_pas_vers_une_route_morte() {
    let v = json("/v1/equides").await;
    let note = v["pagination"]["note"].as_str().unwrap_or("");
    assert!(
        !note.contains("/v1/export"),
        "la note renvoie vers une route retirée : {note}"
    );
}

#[tokio::test]
async fn le_schema_d_erreur_decrit_le_corps_reellement_servi() {
    let doc = json("/openapi.json").await;
    let schema = &doc["components"]["schemas"]["Probleme"];
    assert!(schema.is_object(), "le schéma RFC 9457 doit être décrit");

    let (_, _, corps) = get("/v1/equides?rase=x").await;
    let recu: serde_json::Value = serde_json::from_str(&corps).unwrap();
    let decrites = schema["properties"].as_object().unwrap();

    for champ in recu.as_object().unwrap().keys() {
        assert!(
            decrites.contains_key(champ),
            "le corps d'erreur porte « {champ} », que le schéma ne décrit pas"
        );
    }
    for requis in schema["required"].as_array().unwrap() {
        assert!(
            recu.get(requis.as_str().unwrap()).is_some(),
            "le schéma annonce « {requis} » comme requis, le corps ne le porte pas"
        );
    }
}

#[tokio::test]
async fn le_quota_est_pondere_par_le_cout_de_la_requete() {
    let bon_marche = app();
    for i in 0..45 {
        let (statut, _) = get_sur(&bon_marche, "/v1/equides?limite=100").await;
        assert_eq!(statut, StatusCode::OK, "requête bon marché n° {i} refusée");
    }

    let lourd = app();
    let mut refusees = 0;
    for _ in 0..45 {
        let (statut, _) = get_sur(&lourd, "/v1/equides?limite=100&vue=complet").await;
        if statut == StatusCode::TOO_MANY_REQUESTS {
            refusees += 1;
        }
    }
    assert!(
        refusees > 0,
        "45 requêtes à 5 unités dépassent la rafale de 200 : aucune n'a été refusée"
    );
}

#[tokio::test]
async fn le_refus_de_quota_est_actionnable() {
    let routeur = app();
    let mut vu = None;
    for _ in 0..60 {
        let (statut, entetes) = get_sur(&routeur, "/v1/referentiels/races").await;
        if statut == StatusCode::TOO_MANY_REQUESTS {
            vu = Some(entetes);
            break;
        }
    }
    let entetes = vu.expect("le référentiel coûte 5 unités : la rafale doit finir par céder");
    let attente: u64 = entetes[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .expect("Retry-After en secondes");
    assert!(
        attente >= 1,
        "un Retry-After de 0 invite à réessayer aussitôt"
    );
}

#[tokio::test]
async fn une_requete_sans_adresse_de_pair_est_refusee() {
    let req = Request::builder()
        .uri("/v1/equides")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let reponse = app().oneshot(req).await.unwrap();
    assert_eq!(reponse.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn les_endpoints_ordinaires_restent_a_une_unite() {
    let routeur = app();
    for uri in [
        "/healthz",
        "/v1/meta",
        "/v1/stats",
        "/v1/equides",
        "/v1/search?q=e",
        "/v1/equides/EEEEEEEEEEEEEEEEEEEEEA",
    ] {
        for i in 0..25 {
            let (statut, _) = get_sur(&routeur, uri).await;
            assert_eq!(statut, StatusCode::OK, "{uri} refusé au tour {i}");
        }
    }
}

#[tokio::test]
async fn cors_ouvert_pour_la_reutilisation() {
    let (_, entetes, _) = get_avec("/v1/equides", vec![("origin", "https://exemple.fr")]).await;
    assert_eq!(
        entetes[header::ACCESS_CONTROL_ALLOW_ORIGIN]
            .to_str()
            .unwrap(),
        "*"
    );
}
