pub const NULL_DATE: i32 = i32::MIN;

pub fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}

pub fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn parse_ddmmyyyy(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let b = s.as_bytes();
    if b.len() != 10 || b[2] != b'/' || b[5] != b'/' {
        return None;
    }
    let num = |r: &[u8]| -> Option<u32> {
        let mut v = 0u32;
        for &c in r {
            if !c.is_ascii_digit() {
                return None;
            }
            v = v * 10 + (c - b'0') as u32;
        }
        Some(v)
    };
    let d = num(&b[0..2])?;
    let m = num(&b[3..5])?;
    let y = num(&b[6..10])? as i32;
    if m == 0 || m > 12 || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

pub fn to_iso(days: i32) -> Option<String> {
    if days == NULL_DATE {
        return None;
    }
    let (y, m, d) = civil_from_days(days);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

pub fn parse_iso(s: &str) -> Option<i32> {
    let b = s.trim().as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: &[u8]| -> Option<u32> {
        let mut v = 0u32;
        for &c in r {
            if !c.is_ascii_digit() {
                return None;
            }
            v = v * 10 + (c - b'0') as u32;
        }
        Some(v)
    };
    let y = num(&b[0..4])? as i32;
    let m = num(&b[5..7])?;
    let d = num(&b[8..10])?;
    if m == 0 || m > 12 || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

pub fn year_of(days: i32) -> Option<i32> {
    if days == NULL_DATE {
        return None;
    }
    Some(civil_from_days(days).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_civil() {
        for &(y, m, d) in &[(1970, 1, 1), (1976, 7, 14), (2000, 2, 29), (2025, 12, 31)] {
            let days = days_from_civil(y, m, d);
            assert_eq!(civil_from_days(days), (y, m, d));
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn analyse_format_source() {
        assert_eq!(
            parse_ddmmyyyy("27/04/1987"),
            Some(days_from_civil(1987, 4, 27))
        );
        assert_eq!(
            to_iso(parse_ddmmyyyy("27/04/1987").unwrap()).as_deref(),
            Some("1987-04-27")
        );
        assert_eq!(parse_ddmmyyyy("31/02/1990"), None);
        assert_eq!(parse_ddmmyyyy("00/01/1990"), None);
        assert_eq!(parse_ddmmyyyy("01/13/1990"), None);
        assert_eq!(parse_ddmmyyyy(""), None);
        assert_eq!(parse_ddmmyyyy("1990-01-01"), None);
        assert!(parse_ddmmyyyy("29/02/2000").is_some());
        assert_eq!(parse_ddmmyyyy("29/02/1900"), None);
    }

    #[test]
    fn millesime_aberrant_conserve() {
        let d = parse_ddmmyyyy("01/01/0200").expect("date calendaire valide");
        assert_eq!(year_of(d), Some(200));
        assert_eq!(to_iso(d).as_deref(), Some("0200-01-01"));
    }

    #[test]
    fn date_absente() {
        assert_eq!(to_iso(NULL_DATE), None);
        assert_eq!(year_of(NULL_DATE), None);
    }

    #[test]
    fn analyse_iso() {
        assert_eq!(parse_iso("1987-04-27"), Some(days_from_civil(1987, 4, 27)));
        assert_eq!(parse_iso("1987-4-27"), None);
        assert_eq!(parse_iso("bogus"), None);
    }
}
