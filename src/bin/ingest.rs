use anyhow::{Context, Result, bail};
use clap::Parser;
use equides_api::ids::{self, ID_BYTES};
use equides_api::snapshot::{
    ANNEE_ABSENTE, Anomalies, Arene, COEFFICIENT_ABSENT, DatasetMeta, FORMAT_VERSION,
    SANS_MODALITE, SANS_PARENT, Snapshot,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const PRODUCTEUR: &str = "Institut français du cheval et de l'équitation (IFCE), département SIRE";
const SOURCE: &str = "Extraction des fiches publiques d'infochevaux.ifce.fr";
const URL_MODELE: &str = "https://infochevaux.ifce.fr/fr/{slug}-{id}/infos-generales";

const LICENCE_DEFAUT: &str =
    "Licence Ouverte / réutilisation d'informations publiques (CRPA art. L321-1)";

#[derive(Parser)]
#[command(
    name = "equides-ingest",
    about = "Convertit l'extraction JSONL des équidés en image binaire pour equides-api"
)]
struct Args {
    #[arg(long)]
    source: PathBuf,

    #[arg(long, default_value = "data/equides.bin")]
    sortie: PathBuf,

    #[arg(long, default_value = LICENCE_DEFAUT)]
    licence: String,
}

#[derive(Deserialize)]
struct Ligne {
    id: String,
    slug: Option<String>,
    nom: Option<String>,
    race: Option<String>,
    sexe: Option<String>,
    robe: Option<String>,
    annee_naissance: Option<i32>,
    filiation_texte: Option<String>,
    pere: Option<Parent>,
    mere: Option<Parent>,
    pere_de_mere: Option<Parent>,
    performances: Option<Vec<Performance>>,
}

#[derive(Deserialize)]
struct Parent {
    id: Option<String>,
    nom: Option<String>,
    race: Option<String>,
    pays: Option<String>,
    alias: Option<String>,
}

#[derive(Deserialize)]
struct Performance {
    discipline: Option<String>,
    texte: Option<String>,
    indices: Option<Vec<Indice>>,
}

#[derive(Deserialize)]
struct Indice {
    code: Option<String>,
    valeur: Option<String>,
    coefficient: Option<f64>,
    annee: Option<i32>,
    appreciation: Option<String>,
}

#[derive(Default)]
struct Interneur {
    valeurs: Vec<String>,
    index: HashMap<String, u32>,
}

impl Interneur {
    fn intern(&mut self, v: &str) -> u32 {
        if let Some(&code) = self.index.get(v) {
            return code;
        }
        let code = self.valeurs.len() as u32;
        self.valeurs.push(v.to_string());
        self.index.insert(v.to_string(), code);
        code
    }

    fn intern16(&mut self, v: &str, colonne: &str) -> Result<u16> {
        let code = self.intern(v);
        let code = u16::try_from(code).with_context(|| {
            format!("plus de 65 535 valeurs distinctes dans la colonne `{colonne}`")
        })?;
        anyhow::ensure!(
            code != SANS_MODALITE,
            "colonne `{colonne}` saturée : le code {SANS_MODALITE} est réservé"
        );
        Ok(code)
    }
}

fn slugifier(nom: &str) -> String {
    let mut out = String::with_capacity(nom.len());
    let mut separateur_en_attente = false;
    for c in equides_api::text::fold(nom).chars() {
        if c.is_ascii_alphanumeric() {
            if separateur_en_attente && !out.is_empty() {
                out.push('-');
            }
            separateur_en_attente = false;
            out.push(c.to_ascii_lowercase());
        } else {
            separateur_en_attente = true;
        }
    }
    out
}

#[derive(Default)]
struct Colonnes {
    ids: Vec<u8>,
    race: Vec<u16>,
    robe: Vec<u16>,
    sexe: Vec<u8>,
    annee: Vec<i16>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let debut = Instant::now();

    eprintln!("→ empreinte de {}", args.source.display());
    let empreinte = empreinte_fichier(&args.source)?;
    eprintln!("  blake3 = {empreinte}");

    eprintln!("→ analyse du JSONL");
    let fichier = std::fs::File::open(&args.source)
        .with_context(|| format!("ouverture de {}", args.source.display()))?;
    let lecteur = BufReader::with_capacity(1 << 22, fichier);

    let mut races = Interneur::default();
    let mut races_lien = Interneur::default();
    let mut pays_lien = Interneur::default();
    let mut robes = Interneur::default();
    let mut sexes = Interneur::default();
    let mut disciplines = Interneur::default();
    let mut codes_indice = Interneur::default();
    let mut appreciations = Interneur::default();

    let mut col = Colonnes::default();
    let mut index_id: HashMap<[u8; ID_BYTES], u32> = HashMap::new();
    let mut noms = Arene::nouvelle();
    let mut slugs = Arene::nouvelle();
    let mut filiations = Arene::nouvelle();

    let mut brut_pere: Vec<Option<[u8; ID_BYTES]>> = Vec::new();
    let mut brut_mere: Vec<Option<[u8; ID_BYTES]>> = Vec::new();
    let mut brut_pdm: Vec<Option<[u8; ID_BYTES]>> = Vec::new();
    let mut lien_race: Vec<u16> = Vec::new();
    let mut lien_pays: Vec<u16> = Vec::new();
    let mut lien_alias: Vec<(u32, u8, String)> = Vec::new();
    let mut noms_lien: Vec<Option<String>> = Vec::new();

    let mut perf_bornes: Vec<u32> = vec![0];
    let mut perf_discipline: Vec<u16> = Vec::new();
    let mut perf_textes = Arene::nouvelle();
    let mut ind_bornes: Vec<u32> = vec![0];
    let mut ind_code: Vec<u16> = Vec::new();
    let mut ind_valeurs = Arene::nouvelle();
    let mut ind_coefficient: Vec<f32> = Vec::new();
    let mut ind_annee: Vec<i16> = Vec::new();
    let mut ind_appreciation: Vec<u16> = Vec::new();

    let mut anomalies = Anomalies::default();
    let mut n: u32 = 0;

    for (numero, ligne) in lecteur.lines().enumerate() {
        let ligne = ligne.with_context(|| format!("lecture de la ligne {}", numero + 1))?;
        if ligne.trim().is_empty() {
            continue;
        }
        let r: Ligne = serde_json::from_str(&ligne)
            .with_context(|| format!("JSON invalide ligne {}", numero + 1))?;

        let Some(id) = ids::decode(&r.id) else {
            anomalies.identifiants_illisibles += 1;
            continue;
        };
        if index_id.insert(id, n).is_some() {
            bail!("identifiant en double : {} (ligne {})", r.id, numero + 1);
        }
        col.ids.extend_from_slice(&id);

        let nom = r.nom.as_deref().unwrap_or("");
        let slug = r.slug.as_deref().unwrap_or("");
        if !slug.is_empty() && slug != slugifier(nom) {
            anomalies.slugs_non_deductibles += 1;
        }
        noms.pousser(nom)?;
        slugs.pousser(slug)?;
        filiations.pousser(r.filiation_texte.as_deref().unwrap_or(""))?;

        col.race.push(match r.race.as_deref() {
            Some(v) if !v.is_empty() => races.intern16(v, "race")?,
            _ => SANS_MODALITE,
        });
        col.robe.push(match r.robe.as_deref() {
            Some(v) if !v.is_empty() => robes.intern16(v, "robe")?,
            _ => {
                anomalies.robe_absente += 1;
                SANS_MODALITE
            }
        });
        col.sexe.push(match r.sexe.as_deref() {
            Some(v) if !v.is_empty() => {
                u8::try_from(sexes.intern(v)).context("plus de 255 codes sexe distincts")?
            }
            _ => u8::MAX,
        });
        col.annee.push(if let Some(a) = r.annee_naissance {
            anomalies.annee_naissance_min =
                Some(anomalies.annee_naissance_min.map_or(a, |m| m.min(a)));
            anomalies.annee_naissance_max =
                Some(anomalies.annee_naissance_max.map_or(a, |m| m.max(a)));
            i16::try_from(a).unwrap_or(ANNEE_ABSENTE)
        } else {
            anomalies.annee_naissance_absente += 1;
            ANNEE_ABSENTE
        });

        let mut identifiant: [Option<[u8; ID_BYTES]>; 3] = [None; 3];
        for (genre, lien) in [&r.pere, &r.mere, &r.pere_de_mere].into_iter().enumerate() {
            identifiant[genre] = lien
                .as_ref()
                .and_then(|x| x.id.as_deref())
                .and_then(ids::decode);
            lien_race.push(match lien.as_ref().and_then(|x| x.race.as_deref()) {
                Some(v) if !v.is_empty() => races_lien.intern16(v, "race de lien")?,
                _ => SANS_MODALITE,
            });
            lien_pays.push(match lien.as_ref().and_then(|x| x.pays.as_deref()) {
                Some(v) if !v.is_empty() => pays_lien.intern16(v, "pays de lien")?,
                _ => SANS_MODALITE,
            });
            if let Some(a) = lien.as_ref().and_then(|x| x.alias.as_deref())
                && !a.is_empty()
            {
                lien_alias.push((n, genre as u8, a.to_string()));
            }
            noms_lien.push(
                lien.as_ref()
                    .and_then(|x| x.nom.as_deref())
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
            );
        }
        let [pere, mere, pdm] = identifiant;
        if pere.is_none() && mere.is_none() && pdm.is_none() {
            anomalies.sans_filiation += 1;
        }
        brut_pere.push(pere);
        brut_mere.push(mere);
        brut_pdm.push(pdm);

        for perf in r.performances.into_iter().flatten() {
            perf_discipline.push(match perf.discipline.as_deref() {
                Some(v) if !v.is_empty() => disciplines.intern16(v, "discipline")?,
                _ => SANS_MODALITE,
            });
            perf_textes.pousser(perf.texte.as_deref().unwrap_or(""))?;
            for ind in perf.indices.into_iter().flatten() {
                ind_code.push(match ind.code.as_deref() {
                    Some(v) if !v.is_empty() => codes_indice.intern16(v, "code d'indice")?,
                    _ => SANS_MODALITE,
                });
                ind_valeurs.pousser(ind.valeur.as_deref().unwrap_or(""))?;
                ind_coefficient.push(ind.coefficient.map_or(COEFFICIENT_ABSENT, |c| c as f32));
                ind_annee.push(
                    ind.annee
                        .and_then(|a| i16::try_from(a).ok())
                        .unwrap_or(ANNEE_ABSENTE),
                );
                ind_appreciation.push(match ind.appreciation.as_deref() {
                    Some(v) if !v.is_empty() => appreciations.intern16(v, "appréciation")?,
                    _ => SANS_MODALITE,
                });
            }
            ind_bornes.push(u32::try_from(ind_code.len()).context("trop d'indices")?);
        }
        perf_bornes.push(u32::try_from(perf_discipline.len()).context("trop de performances")?);

        n = n.checked_add(1).context("plus de 4 milliards de lignes")?;
        if n.is_multiple_of(500_000) {
            eprintln!("  {n} lignes… ({:.0?})", debut.elapsed());
        }
    }

    if n == 0 {
        bail!("le fichier ne contient aucune ligne exploitable");
    }
    eprintln!("→ {n} lignes analysées en {:.1?}", debut.elapsed());

    eprintln!("→ résolution du graphe de filiation");
    let mut pendants = 0u32;
    let mut liens_rompus: Vec<(u32, u8, String)> = Vec::new();
    let mut resoudre = |brut: Vec<Option<[u8; ID_BYTES]>>, genre: u8| -> Vec<u32> {
        brut.into_iter()
            .enumerate()
            .map(|(row, id)| {
                let Some(id) = id else {
                    return SANS_PARENT;
                };
                if let Some(&cible) = index_id.get(&id) {
                    return cible;
                }
                pendants += 1;
                if let Some(nom) = noms_lien[row * 3 + genre as usize].take() {
                    liens_rompus.push((row as u32, genre, nom));
                }
                SANS_PARENT
            })
            .collect()
    };
    let col_pere = resoudre(brut_pere, 0);
    let col_mere = resoudre(brut_mere, 1);
    let col_pdm = resoudre(brut_pdm, 2);
    anomalies.parents_pendants = pendants;
    liens_rompus.sort_unstable_by_key(|(row, genre, _)| (*row, *genre));
    lien_alias.sort_unstable_by_key(|(row, genre, _)| (*row, *genre));
    drop((index_id, noms_lien));

    eprintln!(
        "  races={} robes={} sexes={} disciplines={} codes={}",
        races.valeurs.len(),
        robes.valeurs.len(),
        sexes.valeurs.len(),
        disciplines.valeurs.len(),
        codes_indice.valeurs.len()
    );
    eprintln!(
        "  liens : {} races abrégées, {} pays, {} alias, {} liens rompus nommés",
        races_lien.valeurs.len(),
        pays_lien.valeurs.len(),
        lien_alias.len(),
        liens_rompus.len()
    );
    eprintln!(
        "  performances : {} entrées, {} indices",
        perf_discipline.len(),
        ind_code.len()
    );
    eprintln!(
        "  anomalies : {} sans filiation, {} parents pendants, {} sans millésime, {} sans robe, {} slugs non déductibles",
        anomalies.sans_filiation,
        anomalies.parents_pendants,
        anomalies.annee_naissance_absente,
        anomalies.robe_absente,
        anomalies.slugs_non_deductibles
    );
    if let (Some(min), Some(max)) = (anomalies.annee_naissance_min, anomalies.annee_naissance_max) {
        eprintln!("  millésimes : {min} → {max}");
    }

    let snap = Snapshot {
        format_version: FORMAT_VERSION,
        meta: DatasetMeta {
            producteur: PRODUCTEUR.to_string(),
            source: SOURCE.to_string(),
            licence: args.licence,
            url_modele: URL_MODELE.to_string(),
            ingere_le: aujourdhui_iso(),
            empreinte_source: empreinte,
            lignes: n,
            anomalies,
        },
        dict_races: races.valeurs,
        dict_robes: robes.valeurs,
        dict_sexes: sexes.valeurs,
        dict_races_lien: races_lien.valeurs,
        dict_pays_lien: pays_lien.valeurs,
        dict_disciplines: disciplines.valeurs,
        dict_codes_indice: codes_indice.valeurs,
        dict_appreciations: appreciations.valeurs,
        ids: col.ids,
        noms,
        slugs,
        filiations,
        race: col.race,
        robe: col.robe,
        sexe: col.sexe,
        annee: col.annee,
        pere: col_pere,
        mere: col_mere,
        pere_de_mere: col_pdm,
        lien_race,
        lien_pays,
        lien_alias,
        liens_rompus,
        perf_bornes,
        perf_discipline,
        perf_textes,
        ind_bornes,
        ind_code,
        ind_valeurs,
        ind_coefficient,
        ind_annee,
        ind_appreciation,
    };

    if let Some(parent) = dossier_parent(&args.sortie) {
        std::fs::create_dir_all(&parent)
            .with_context(|| format!("création du dossier {}", parent.display()))?;
    }
    eprintln!("→ écriture de {}", args.sortie.display());
    let taille = snap.write(&args.sortie)?;
    eprintln!(
        "✓ image de {:.1} Mio écrite en {:.1?}",
        taille as f64 / (1024.0 * 1024.0),
        debut.elapsed()
    );
    Ok(())
}

fn dossier_parent(chemin: &Path) -> Option<PathBuf> {
    chemin
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
}

fn empreinte_fichier(chemin: &Path) -> Result<String> {
    let mut f = std::fs::File::open(chemin)
        .with_context(|| format!("ouverture de {}", chemin.display()))?;
    let mut h = blake3::Hasher::new();
    let mut tampon = vec![0u8; 1 << 22];
    loop {
        let lus = f.read(&mut tampon)?;
        if lus == 0 {
            break;
        }
        h.update(&tampon[..lus]);
    }
    Ok(h.finalize().to_hex().to_string())
}

fn aujourdhui_iso() -> String {
    let secondes = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    equides_api::date::to_iso((secondes / 86_400) as i32)
        .unwrap_or_else(|| "1970-01-01".to_string())
}
