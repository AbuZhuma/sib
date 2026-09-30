#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PressureLine {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pressure {
    pub some: PressureLine,
    pub full: Option<PressureLine>,
}

pub fn parse(raw: &str) -> Option<Pressure> {
    let mut some = None;
    let mut full = None;
    for line in raw.lines() {
        let (kind, rest) = line.split_once(' ')?;
        let parsed = parse_line(rest)?;
        match kind {
            "some" => some = Some(parsed),
            "full" => full = Some(parsed),
            _ => {}
        }
    }
    Some(Pressure { some: some?, full })
}

fn parse_line(rest: &str) -> Option<PressureLine> {
    let field = |name: &str| {
        rest.split_whitespace()
            .find_map(|part| part.strip_prefix(name)?.strip_prefix('='))
            .and_then(|v| v.parse::<f64>().ok())
    };
    Some(PressureLine {
        avg10: field("avg10")?,
        avg60: field("avg60")?,
        avg300: field("avg300")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_some_and_full_lines() {
        let raw = "some avg10=0.35 avg60=0.30 avg300=0.23 total=65389393\nfull avg10=0.10 avg60=0.20 avg300=0.22 total=45994103";
        let pressure = parse(raw).expect("parse");
        assert_eq!(pressure.some.avg10, 0.35);
        assert_eq!(pressure.full.map(|f| f.avg60), Some(0.20));
    }

    #[test]
    fn parse_empty_is_none() {
        assert_eq!(parse(""), None);
    }
}
