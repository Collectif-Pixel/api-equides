use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

pub const FORMAT_VERSION: u32 = 6;

pub const SANS_PARENT: u32 = u32::MAX;
pub const SANS_MODALITE: u16 = u16::MAX;
pub const ANNEE_ABSENTE: i16 = i16::MIN;
pub const COEFFICIENT_ABSENT: f32 = f32::NAN;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Anomalies {
    pub annee_naissance_absente: u32,
    pub annee_naissance_min: Option<i32>,
    pub annee_naissance_max: Option<i32>,
    pub robe_absente: u32,
    pub sans_filiation: u32,
    pub parents_references: u32,
    pub parents_pendants: u32,
    pub references_de_parents: u32,
    pub references_pendantes: u32,
    pub identifiants_illisibles: u32,
    pub slugs_non_deductibles: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMeta {
    pub producteur: String,
    pub source: String,
    pub licence: String,
    pub url_modele: String,
    pub ingere_le: String,
    pub fichier_source: String,
    pub octets_source: u64,
    pub algorithme_empreinte: String,
    pub empreinte_source: String,
    pub lignes: u32,
    pub anomalies: Anomalies,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Arene {
    pub octets: Vec<u8>,
    pub bornes: Vec<u32>,
}

impl Arene {
    pub fn nouvelle() -> Self {
        Self {
            octets: Vec::new(),
            bornes: vec![0],
        }
    }

    pub fn pousser(&mut self, s: &str) -> Result<()> {
        self.octets.extend_from_slice(s.as_bytes());
        self.bornes
            .push(u32::try_from(self.octets.len()).context("arène au-delà de 4 Gio")?);
        Ok(())
    }

    #[inline]
    pub fn get(&self, i: usize) -> &str {
        let (d, f) = (self.bornes[i] as usize, self.bornes[i + 1] as usize);
        std::str::from_utf8(&self.octets[d..f]).unwrap_or("")
    }

    pub fn len(&self) -> usize {
        self.bornes.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub format_version: u32,
    pub meta: DatasetMeta,

    pub dict_races: Vec<String>,
    pub dict_robes: Vec<String>,
    pub dict_sexes: Vec<String>,
    pub dict_races_lien: Vec<String>,
    pub dict_pays_lien: Vec<String>,
    pub dict_disciplines: Vec<String>,
    pub dict_codes_indice: Vec<String>,
    pub dict_appreciations: Vec<String>,
    pub dict_statuts_reproducteur: Vec<String>,

    pub ids: Vec<u8>,
    pub noms: Arene,
    pub slugs: Arene,
    pub filiations: Arene,
    pub records: Arene,

    pub race: Vec<u16>,
    pub robe: Vec<u16>,
    pub sexe: Vec<u8>,
    pub annee: Vec<i16>,
    pub statut_reproducteur: Vec<u16>,

    pub pere: Vec<u32>,
    pub mere: Vec<u32>,
    pub pere_de_mere: Vec<u32>,

    pub lien_race: Vec<u16>,
    pub lien_pays: Vec<u16>,
    pub lien_alias: Vec<(u32, u8, String)>,
    pub liens_rompus: Vec<(u32, u8, String)>,

    pub perf_bornes: Vec<u32>,
    pub perf_discipline: Vec<u16>,
    pub perf_textes: Arene,
    pub ind_bornes: Vec<u32>,
    pub ind_code: Vec<u16>,
    pub ind_valeurs: Arene,
    pub ind_coefficient: Vec<f32>,
    pub ind_annee: Vec<i16>,
    pub ind_appreciation: Vec<u16>,
}

impl Snapshot {
    pub fn write(&self, path: &Path) -> Result<u64> {
        let file = std::fs::File::create(path)
            .with_context(|| format!("création de l'image {}", path.display()))?;
        let mut w = BufWriter::with_capacity(1 << 22, file);
        postcard::to_io(self, &mut w).context("sérialisation de l'image")?;
        w.flush().context("vidage du tampon d'écriture")?;
        drop(w);
        Ok(std::fs::metadata(path)?.len())
    }

    pub fn read(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).with_context(|| {
            format!(
                "ouverture de l'image {} — lancez d'abord `equides-ingest`",
                path.display()
            )
        })?;
        let mut r = BufReader::with_capacity(1 << 22, file);
        let mut octets = Vec::new();
        r.read_to_end(&mut octets)
            .with_context(|| format!("lecture de l'image {}", path.display()))?;
        let snap: Snapshot = postcard::from_bytes(&octets).context("désérialisation de l'image")?;
        drop(octets);

        anyhow::ensure!(
            snap.format_version == FORMAT_VERSION,
            "image en version de format {} alors que ce binaire attend la version {} ; \
             relancez `equides-ingest`",
            snap.format_version,
            FORMAT_VERSION
        );
        snap.verifier_coherence()?;
        Ok(snap)
    }

    fn verifier_coherence(&self) -> Result<()> {
        let n = self.meta.lignes as usize;
        let colonne = |nom: &str, len: usize| -> Result<()> {
            anyhow::ensure!(
                len == n,
                "colonne `{nom}` de longueur {len}, {n} attendues (image corrompue)"
            );
            Ok(())
        };
        colonne("race", self.race.len())?;
        colonne("robe", self.robe.len())?;
        colonne("sexe", self.sexe.len())?;
        colonne("annee", self.annee.len())?;
        colonne("statut_reproducteur", self.statut_reproducteur.len())?;
        colonne("pere", self.pere.len())?;
        colonne("mere", self.mere.len())?;
        colonne("pere_de_mere", self.pere_de_mere.len())?;
        anyhow::ensure!(
            self.lien_race.len() == n * 3 && self.lien_pays.len() == n * 3,
            "colonnes d'attributs de lien de longueur {} et {}, {} attendues (image corrompue)",
            self.lien_race.len(),
            self.lien_pays.len(),
            n * 3
        );
        colonne("noms", self.noms.len())?;
        colonne("slugs", self.slugs.len())?;
        colonne("filiations", self.filiations.len())?;
        colonne("records", self.records.len())?;

        anyhow::ensure!(
            self.ids.len() == n * crate::ids::ID_BYTES,
            "table d'identifiants de {} octets, {} attendus (image corrompue)",
            self.ids.len(),
            n * crate::ids::ID_BYTES
        );
        anyhow::ensure!(
            self.perf_bornes.len() == n + 1,
            "`perf_bornes` de longueur {}, {} attendues (image corrompue)",
            self.perf_bornes.len(),
            n + 1
        );

        let nb_perf = self.perf_discipline.len();
        anyhow::ensure!(
            self.perf_bornes.last().copied().unwrap_or(0) as usize == nb_perf,
            "bornes de performances incohérentes avec la table (image corrompue)"
        );
        anyhow::ensure!(
            self.ind_bornes.len() == nb_perf + 1,
            "`ind_bornes` de longueur {}, {} attendues (image corrompue)",
            self.ind_bornes.len(),
            nb_perf + 1
        );
        anyhow::ensure!(
            self.ind_bornes.last().copied().unwrap_or(0) as usize == self.ind_code.len(),
            "bornes d'indices incohérentes avec la table (image corrompue)"
        );

        for (nom, colonne) in [
            ("pere", &self.pere),
            ("mere", &self.mere),
            ("pere_de_mere", &self.pere_de_mere),
        ] {
            if let Some(mauvais) = colonne
                .iter()
                .find(|&&r| r != SANS_PARENT && r as usize >= n)
            {
                anyhow::bail!(
                    "colonne `{nom}` référence la ligne {mauvais}, hors des {n} lignes (image corrompue)"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arene_restitue_les_chaines() {
        let mut a = Arene::nouvelle();
        for s in ["QABALAH MERCURY", "", "ÉCLAIR"] {
            a.pousser(s).unwrap();
        }
        assert_eq!(a.len(), 3);
        assert_eq!(a.get(0), "QABALAH MERCURY");
        assert_eq!(a.get(1), "");
        assert_eq!(a.get(2), "ÉCLAIR");
    }

    #[test]
    fn arene_vide() {
        let a = Arene::nouvelle();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
    }
}
