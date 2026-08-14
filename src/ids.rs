use data_encoding::{Encoding, Specification};
use std::sync::LazyLock;

pub const ID_BYTES: usize = 16;
pub const ID_CHARS: usize = 22;

static BASE64URL: LazyLock<Encoding> = LazyLock::new(|| {
    let mut spec = Specification::new();
    spec.symbols
        .push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_");
    spec.check_trailing_bits = false;
    spec.encoding().expect("spécification base64url valide")
});

pub fn decode(s: &str) -> Option<[u8; ID_BYTES]> {
    if s.len() != ID_CHARS {
        return None;
    }
    let octets = BASE64URL.decode(s.as_bytes()).ok()?;
    let mut out = [0u8; ID_BYTES];
    if octets.len() != ID_BYTES {
        return None;
    }
    out.copy_from_slice(&octets);
    Some(out)
}

pub fn encode(id: &[u8; ID_BYTES]) -> String {
    BASE64URL.encode(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REELS: [&str; 4] = [
        "Z4ogLhlkS2CeUdq0bZ0YFw",
        "cIw2F_VxQDi1i9EqjsXH8w",
        "gecpqPj6Rc2vy_Hz_Y1DmQ",
        "GRkl_91jQUuiWZ2Vfjz0eg",
    ];

    #[test]
    fn aller_retour_sur_identifiants_reels() {
        for s in REELS {
            let id = decode(s).unwrap_or_else(|| panic!("« {s} » doit se décoder"));
            assert_eq!(
                encode(&id),
                s,
                "l'encodage doit restituer la forme d'origine"
            );
        }
    }

    #[test]
    fn taille_decodee() {
        assert_eq!(decode(REELS[0]).unwrap().len(), 16);
    }

    #[test]
    fn identifiants_distincts_restent_distincts() {
        let a = decode(REELS[0]).unwrap();
        let b = decode(REELS[1]).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn formes_invalides_rejetees() {
        assert_eq!(decode(""), None);
        assert_eq!(decode("trop-court"), None);
        assert_eq!(decode(&"A".repeat(23)), None);
        assert_eq!(decode("Z4ogLhlkS2CeUdq0bZ0Y+w"), None);
    }

    #[test]
    fn les_identifiants_reels_sont_canoniques() {
        for s in REELS {
            assert!(
                matches!(s.chars().last(), Some('A' | 'Q' | 'g' | 'w')),
                "« {s} » devrait finir par un caractère sans bits de queue"
            );
        }
    }

    #[test]
    fn forme_non_canonique_acceptee_puis_normalisee() {
        let non_canonique = "EEEEEEEEEEEEEEEEEEEEEE";
        let id = decode(non_canonique).expect("décodage tolérant");
        assert_eq!(encode(&id), "EEEEEEEEEEEEEEEEEEEEEA");
        assert_eq!(decode("EEEEEEEEEEEEEEEEEEEEEA"), Some(id));
    }

    #[test]
    fn alphabet_sans_ambiguite_url() {
        let encode_tout = encode(&[0xff; ID_BYTES]);
        assert!(!encode_tout.contains('+') && !encode_tout.contains('/'));
    }
}
