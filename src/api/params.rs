use super::erreur::Erreur;
use crate::query::{Filtres, LIMITE_DEFAUT, LIMITE_MAX, OFFSET_MAX, Sens, Tri};

pub struct Params(Vec<(String, String)>);

impl Params {
    pub fn analyser(query: Option<&str>) -> Result<Self, Erreur> {
        super::protection::verifier_taille_requete(query)?;
        Ok(Self::parse(query))
    }

    pub fn parse(query: Option<&str>) -> Self {
        let brut = query.unwrap_or("");
        Self(
            form_urlencoded::parse(brut.as_bytes())
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .filter(|(_, v)| !v.trim().is_empty())
                .collect(),
        )
    }

    pub fn premier(&self, cle: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k == cle).map(|(_, v)| v.trim())
    }

    pub fn toutes(&self, cle: &str) -> Vec<String> {
        self.0
            .iter()
            .filter(|(k, _)| k == cle)
            .flat_map(|(_, v)| v.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn booleen(&self, cle: &str) -> Result<Option<bool>, Erreur> {
        match self.premier(cle) {
            None => Ok(None),
            Some(v) => match v.to_ascii_lowercase().as_str() {
                "true" | "1" | "oui" | "o" => Ok(Some(true)),
                "false" | "0" | "non" | "n" => Ok(Some(false)),
                autre => Err(Erreur::parametre_invalide(
                    cle,
                    format!("`{cle}` attend un booléen, reçu « {autre} »"),
                    "valeurs admises : true, false, 1, 0, oui, non",
                )),
            },
        }
    }

    pub fn entier(&self, cle: &str) -> Result<Option<usize>, Erreur> {
        match self.premier(cle) {
            None => Ok(None),
            Some(v) => v.parse::<usize>().map(Some).map_err(|_| {
                Erreur::parametre_invalide(
                    cle,
                    format!("`{cle}` attend un entier positif, reçu « {v} »"),
                    "exemple : 50",
                )
            }),
        }
    }

    pub fn annees(&self, cle: &str) -> Result<Vec<i32>, Erreur> {
        self.toutes(cle)
            .into_iter()
            .map(|v| {
                v.parse::<i32>().map_err(|_| {
                    Erreur::parametre_invalide(
                        cle,
                        format!("`{cle}` attend un millésime sur quatre chiffres, reçu « {v} »"),
                        "exemple : 2010",
                    )
                })
            })
            .collect()
    }

    pub fn refuser_inconnus(&self, admis: &[&str]) -> Result<(), Erreur> {
        if let Some((cle, _)) = self.0.iter().find(|(k, _)| !admis.contains(&k.as_str())) {
            let proche = suggerer(cle, admis)
                .map(|a| format!("vouliez-vous dire `{a}` ? "))
                .unwrap_or_default();
            return Err(Erreur::parametre_invalide(
                cle,
                format!("paramètre inconnu `{cle}`"),
                format!("{proche}paramètres admis : {}", admis.join(", ")),
            ));
        }
        Ok(())
    }

    pub fn filtres(&self) -> Result<Filtres, Erreur> {
        Ok(Filtres {
            races: self.toutes("race"),
            robes: self.toutes("robe"),
            sexes: self.toutes("sexe"),
            statuts_reproducteur: self.toutes("statut_reproducteur"),
            disciplines: self.toutes("discipline"),
            codes_indice: self.toutes("indice"),
            annees_naissance: self.annees("annee_naissance")?,
            nom: self.premier("nom").map(std::string::ToString::to_string),
            annee_min: self.annee_simple("annee_min")?,
            annee_max: self.annee_simple("annee_max")?,
            avec_performances: self.booleen("avec_performances")?,
        })
    }

    fn annee_simple(&self, cle: &str) -> Result<Option<i32>, Erreur> {
        match self.premier(cle) {
            None => Ok(None),
            Some(v) => v.parse::<i32>().map(Some).map_err(|_| {
                Erreur::parametre_invalide(
                    cle,
                    format!("`{cle}` attend un millésime, reçu « {v} »"),
                    "exemple : 2010",
                )
            }),
        }
    }

    pub fn tri(&self) -> Result<(Tri, Sens), Erreur> {
        match self.premier("tri") {
            None => Ok((Tri::Naturel, Sens::Croissant)),
            Some(v) => Tri::parse(v).ok_or_else(|| {
                Erreur::parametre_invalide(
                    "tri",
                    format!("tri inconnu « {v} »"),
                    "valeurs admises : naturel, nom, annee — \
                     préfixez par `-` pour l'ordre décroissant (ex. `-annee`)",
                )
            }),
        }
    }

    pub fn pagination(&self) -> Result<(usize, usize), Erreur> {
        let limite = self.entier("limite")?.unwrap_or(LIMITE_DEFAUT);
        if limite == 0 {
            return Err(Erreur::parametre_invalide(
                "limite",
                "`limite` doit valoir au moins 1",
                format!("valeur maximale : {LIMITE_MAX}"),
            ));
        }
        if limite > LIMITE_MAX {
            return Err(Erreur::parametre_invalide(
                "limite",
                format!("`limite` plafonnée à {LIMITE_MAX}, reçu {limite}"),
                "pour extraire l'intégralité du jeu, téléchargez le fichier publié",
            ));
        }
        let offset = self.entier("offset")?.unwrap_or(0);
        if offset > OFFSET_MAX {
            return Err(Erreur::parametre_invalide(
                "offset",
                format!("`offset` plafonné à {OFFSET_MAX}, reçu {offset}"),
                "la pagination profonde dégrade le service ; pour parcourir \
                 l'intégralité du jeu, téléchargez le fichier publié",
            ));
        }
        Ok((limite, offset))
    }
}

fn suggerer<'a>(cle: &str, admis: &[&'a str]) -> Option<&'a str> {
    if cle.is_empty() {
        return None;
    }
    admis
        .iter()
        .map(|candidat| {
            let score = if candidat.starts_with(cle) || cle.starts_with(candidat) {
                0
            } else {
                distance_edition(cle, candidat)
            };
            (score, *candidat)
        })
        .filter(|(score, _)| *score <= 2)
        .min_by_key(|(score, candidat)| (*score, candidat.len()))
        .map(|(_, candidat)| candidat)
}

fn distance_edition(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut precedente: Vec<usize> = (0..=b.len()).collect();
    let mut courante = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        courante[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cout = usize::from(!ca.eq_ignore_ascii_case(cb));
            courante[j + 1] = (precedente[j + 1] + 1)
                .min(courante[j] + 1)
                .min(precedente[j] + cout);
        }
        std::mem::swap(&mut precedente, &mut courante);
    }
    precedente[b.len()]
}

pub const PARAMS_FILTRE: &[&str] = &[
    "race",
    "robe",
    "sexe",
    "statut_reproducteur",
    "discipline",
    "indice",
    "annee_naissance",
    "nom",
    "annee_min",
    "annee_max",
    "avec_performances",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parametre_repete_donne_plusieurs_valeurs() {
        let p = Params::parse(Some("race=PUR+SANG&race=TROTTEUR+FR."));
        assert_eq!(p.toutes("race"), vec!["PUR SANG", "TROTTEUR FR."]);
    }

    #[test]
    fn liste_separee_par_virgules_acceptee() {
        let p = Params::parse(Some("sexe=M,F"));
        assert_eq!(p.toutes("sexe"), vec!["M", "F"]);
    }

    #[test]
    fn valeurs_vides_ignorees() {
        let p = Params::parse(Some("nom=&race=PUR+SANG"));
        assert_eq!(p.premier("nom"), None);
        assert_eq!(p.toutes("race"), vec!["PUR SANG"]);
    }

    #[test]
    fn booleens_en_francais_et_en_anglais() {
        let p = Params::parse(Some("a=true&b=non&c=1&d=0&e=peut-etre"));
        assert_eq!(p.booleen("a").unwrap(), Some(true));
        assert_eq!(p.booleen("b").unwrap(), Some(false));
        assert_eq!(p.booleen("c").unwrap(), Some(true));
        assert_eq!(p.booleen("d").unwrap(), Some(false));
        assert!(p.booleen("e").is_err());
        assert_eq!(p.booleen("absent").unwrap(), None);
    }

    #[test]
    fn parametre_inconnu_rejete_avec_suggestion() {
        let p = Params::parse(Some("rac=PUR+SANG"));
        let err = p.refuser_inconnus(PARAMS_FILTRE).unwrap_err();
        assert_eq!(err.corps.parametre.as_deref(), Some("rac"));
        assert!(err.corps.indice.unwrap().contains("`race`"));
    }

    #[test]
    fn suggestion_sur_faute_de_frappe() {
        for faute in ["rase", "rcae", "raec", "Race"] {
            assert_eq!(
                suggerer(faute, PARAMS_FILTRE),
                Some("race"),
                "« {faute} » doit suggérer `race`"
            );
        }
        assert_eq!(suggerer("sexes", PARAMS_FILTRE), Some("sexe"));
        assert_eq!(suggerer("noms", PARAMS_FILTRE), Some("nom"));
    }

    #[test]
    fn pas_de_suggestion_douteuse() {
        assert_eq!(suggerer("xyzzy", PARAMS_FILTRE), None);
        assert_eq!(suggerer("couleur", PARAMS_FILTRE), None);
        assert_eq!(suggerer("", PARAMS_FILTRE), None);
    }

    #[test]
    fn distance_edition_de_base() {
        assert_eq!(distance_edition("race", "race"), 0);
        assert_eq!(distance_edition("rase", "race"), 1);
        assert_eq!(distance_edition("rcae", "race"), 2);
        assert_eq!(distance_edition("", "race"), 4);
        assert_eq!(distance_edition("race", ""), 4);
        assert_eq!(distance_edition("RACE", "race"), 0);
    }

    #[test]
    fn parametres_admis_acceptes() {
        let p = Params::parse(Some("race=PUR+SANG&nom=felix"));
        assert!(p.refuser_inconnus(PARAMS_FILTRE).is_ok());
    }

    #[test]
    fn pagination_plafonnee() {
        assert!(Params::parse(Some("limite=201")).pagination().is_err());
        assert!(Params::parse(Some("limite=0")).pagination().is_err());
        assert!(Params::parse(Some("offset=10001")).pagination().is_err());
        assert_eq!(
            Params::parse(Some("limite=50&offset=10"))
                .pagination()
                .unwrap(),
            (50, 10)
        );
        assert_eq!(
            Params::parse(None).pagination().unwrap(),
            (LIMITE_DEFAUT, 0)
        );
    }

    #[test]
    fn message_de_plafond_oriente_vers_le_fichier_publie() {
        let err = Params::parse(Some("offset=99999"))
            .pagination()
            .unwrap_err();
        assert!(err.corps.indice.unwrap().contains("fichier publié"));
    }

    #[test]
    fn tri_analyse() {
        assert_eq!(
            Params::parse(None).tri().unwrap(),
            (Tri::Naturel, Sens::Croissant)
        );
        assert_eq!(
            Params::parse(Some("tri=-annee")).tri().unwrap(),
            (Tri::Annee, Sens::Decroissant)
        );
        assert!(Params::parse(Some("tri=couleur")).tri().is_err());
    }
}
