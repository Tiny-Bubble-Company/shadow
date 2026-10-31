//! Street addresses and organisation names with a legal form, across the supported locales.
//!
//! Addresses need a street-type word *and* a house number (or a postcode) in the shape used
//! by that language: `221B Baker Street`, `Hauptstraße 5`, `Zaralaan 93`, `12 rue de la Paix`,
//! `Calle Mayor 3`, `Via Roma 10`, `Rua Augusta, 120`, `48/67 Misra Road`, `東京都…1丁目2番3号`.
//! A match is then extended over trailing `, postcode city` / `, City, ST 12345` parts.

use std::sync::LazyLock;

use regex::Regex;

use super::{Detector, Finding, Kind};

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect("valid regex"));
    };
}

// Capitalised word (allows internal apostrophes, hyphens and dots: "O'Connell", "St.").
const CAP: &str = r"\p{Lu}[\p{L}'’.\-]*";
// Lowercase connecting words inside street names.
const LINK: &str = r"(?:de|del|della|delle|dei|degli|di|da|das|do|dos|du|des|la|le|les|los|las|el|en|e|y|van|von|der|den|het|am|an|im|zum|zur|of|the|d'|l')";

// English: number + Capitalised words + street type.
static EN: LazyLock<Regex> = LazyLock::new(|| {
    let types = "street|avenue|road|boulevard|lane|drive|court|place|square|terrace|parkway|crescent|close|grove|gardens|mews|highway|circle|trail|alley|plaza|viaduct|shoals?|lodge|radial|ridge|heights|harbor|harbour|haven|hollow|meadows?|manor|orchard|pike|turnpike|expressway|freeway|causeway|estates?|falls?|extensions?|junction|landing|loop|summit|tunnel|valley|vista|walk|row|parade|esplanade|promenade|wharf|quay|cove|creek|crossing|bypass|motorway|pass|path|point|port|ranch|spur|station|village|ville|wells|springs?|mount|mountain|island|isle|gateway|glen|knolls?|bridge|brook|burg|camp|canyon|cape|centre|center|cliffs?|common|corners?|course|crest|curve|dale|dam|divide|ferry|field|fields|flats?|ford|forest|forge|fork|forks|fort|garden|gate|hill|hills|inlet|keys?|lakes?|lights?|locks?|mall|mill|mills|mission|neck|oval|overpass|park|parks|pines?|plains?|prairie|ramp|rapids?|rest|ridges|route|shores?|skyway|stream|throughway|trace|track|union|unions|underpass|views?|ways?|chase|drove|green|rise|yard|end|side|run|club|crossroads?|branch|loaf|land|river|stravenue|trafficway|wall|cross|passage|glens|greens|harbors|islands|manors|mountains|parkways|plazas";
    Regex::new(&format!(
        r"(?:\b(?i:flat|apt\.?|apartment|suite|unit|studio)[^\S\n]+\d+[A-Za-z]?,[^\S\n]+|\b\d{{1,5}}[A-Za-z]?(?:[-/]\d{{1,4}}[A-Za-z]?)?,?[^\S\n]+)(?:{CAP}[^\S\n]+){{1,4}}(?i:(?:{types})s?\b|(?:St|Ave|Rd|Blvd|Ln|Dr|Ct|Pl|Sq|Ter|Pkwy|Hwy|Cir|Trl|Cres|Gdns)\b\.?)(?:,?[^\S\n]+(?i:apt\.?|suite|unit|ste\.?|#)[^\S\n]*\d+[a-z]?)?"
    ))
    .expect("valid regex")
});

// German/Dutch/Swedish compound street names + number: "Hauptstraße 5", "Zaralaan 93a",
// "Storgatan 4", "Am Markt 3", and the number-first form "7 Markboulevard", "33 Villagränd".
const COMPOUND_SUFFIXES: &str = "straße|strasse|str\\.|weg|allee|platz|gasse|damm|ufer|chaussee|steig|markt|graben|stieg|berg|wall|tor|kamp|hof|ring|straat|laan|baan|boulevard|plein|singel|gracht|kade|dijk|dreef|steeg|plantsoen|kanaal|haven|wal|veld|hoek|erf|gatan|gata|vägen|väg|gränd|gränden|torget|torg|stigen|backen|backe|allén|allé|leden|kajen|stranden|parken|platsen|esplanaden|promenaden";
static COMPOUND: LazyLock<Regex> = LazyLock::new(|| {
    let suffixes = COMPOUND_SUFFIXES;
    Regex::new(&format!(
        r"\b\p{{Lu}}[\p{{L}}\-]*?(?i:{suffixes})[^\S\n]+\d{{1,4}}[a-zA-Z]?(?:[^\S\n]?[-/][^\S\n]?\d{{1,4}}[a-z]?)?\b|\b\d{{1,5}}[a-zA-Z]?(?:/\d{{1,4}})?,?[^\S\n]+\p{{Lu}}[\p{{L}}\-]*?(?i:{suffixes})\b|\b(?:{CAP}[^\S\n]+)(?:Straße|Strasse|Str\.|Weg|Allee|Platz|Gasse|Ring|Damm|Ufer|Chaussee|Markt|Park|Straat|Laan|Plein|Weg)[^\S\n]+\d{{1,4}}[a-zA-Z]?\b|\b(?:Am|An der|Auf dem|Im|Zum|Zur|In der)[^\S\n]+{CAP}[^\S\n]+\d{{1,4}}[a-z]?\b"
    ))
    .expect("valid regex")
});

// French: "12 bis, rue de la Paix", "44, boulevard Martine Roche".
static FR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\b(?:\d{{1,4}}(?:[^\S\n]?(?:bis|ter))?,?[^\S\n]+)?(?i:rue|avenue|av\.|boulevard|bd|chemin|allée|place|impasse|quai|route|cours|square|voie|passage|sentier|résidence|faubourg|parvis|esplanade|rond-point|villa|cité|hameau|lieu-dit)[^\S\n]+(?:(?:{LINK})[^\S\n]*)*{CAP}(?:[^\S\n]+(?:(?:{LINK})[^\S\n]+)*{CAP}){{0,4}}"
    ))
    .expect("valid regex")
});

// Spanish / Italian / Portuguese: type + name + number.
const ROMANCE_TYPES: &str = "Calle|C/|C\\.|Acceso|Pasadizo|Avenida|Avda\\.?|Av\\.|Paseo|Plaza|Pza\\.|Camino|Carretera|Ctra\\.|Callejón|Cuesta|Vial|Ronda|Travesía|Glorieta|Urbanización|Rambla|Pasaje|Alameda|Cañada|Bajada|Subida|Partida|Vía|Via|Viale|Piazza|Piazzale|Corso|Largo|Vicolo|Strada|Contrada|Borgo|Canale|Rotonda|Lungomare|Stretto|Incrocio|Vico|Salita|Rua|Travessa|Praça|Rodovia|Estrada|Viela|Ladeira|Quadra|Setor|Conjunto|Residencial|Núcleo|Chácara|Fazenda|Jardim|Loteamento|Parque|Passarela|Recanto|Sítio|Trecho|Vereda|Condomínio|Esplanada|Feira|Lagoa|Vale|Campo|Colônia|Distrito|Favela|Morro|Pátio|Vila|Lago|Aeroporto|Avenue";
static ROMANCE: LazyLock<Regex> = LazyLock::new(|| {
    let types = ROMANCE_TYPES;
    Regex::new(&format!(
        r"\b(?:{types})[^\S\n]+(?:(?:{LINK})[^\S\n]+)*{CAP}(?:[^\S\n]+(?:(?:{LINK})[^\S\n]+)*{CAP}){{0,4}}[^\S\n]*,?[^\S\n]+(?:n[º°o]\.?[^\S\n]*)?\d{{1,5}}[A-Za-z]?\b(?:[^\S\n]*,?[^\S\n]*(?i:apt\.?|apartamento|appartamento|piso|puerta|planta|sala|casa|bloco|lote|interno|scala|andar|esc\.?)[^\S\n]*\d+[A-Za-z]?)?"
    ))
    .expect("valid regex")
});

// The same street types with the number first, as in internationalised records:
// "83 Via Brancaccio", "151 Rotonda Osvaldo", "54 Paseo de Macario Ureña".
static ROMANCE_NUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\b\d{{1,5}}[A-Za-z]?,?[^\S\n]+(?:{ROMANCE_TYPES})[^\S\n]+(?:(?:{LINK})[^\S\n]+)*{CAP}(?:[^\S\n]+(?:(?:{LINK})[^\S\n]+)*{CAP}){{0,3}}"
    ))
    .expect("valid regex")
});

// India: "48/67, Misra Road", "H.No. 12, Gandhi Nagar".
static IN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"\b(?:H\.?[^\S\n]?No\.?[^\S\n]*)?\d{{1,4}}(?:/\d{{1,4}})?,[^\S\n]+{CAP}(?:[^\S\n]+{CAP}){{0,3}}"
    ))
    .expect("valid regex")
});
const IN_TYPES: &[&str] = &[
    "Chowk", "Nagar", "Marg", "Road", "Path", "Zila", "Ganj", "Street", "Colony", "Circle",
    "Puram", "Bagh", "Vihar", "Enclave", "Layout", "Lane", "Gali", "Halli", "Peth", "Wadi",
    "Bazar", "Bazaar", "Mohalla", "Sector", "Block", "Phase", "Cross", "Main",
];

// Japan: prefecture + municipality + chōme-ban-gō, optional building.
re!(
    JP,
    r"(?:北海道|東京都|大阪府|京都府|\p{Han}{2,3}県)[\p{Han}\p{Hiragana}\p{Katakana}ー々]{1,24}?\d{1,3}丁目\d{1,4}番(?:地)?\d{1,4}号?(?:[^\S\n]?[\p{Han}\p{Katakana}ー々]+\d{1,4}号?室?)?"
);

// A trailing ", postcode city" / ", City" / " ST 12345" part of an address.
static SEGMENT: LazyLock<Regex> = LazyLock::new(|| {
    let postcode = r"(?:\d{3}[^\S\n]\d{2}|\d{5}-?\d{3}|\d{4}[^\S\n]?[A-Z]{2}|\d{4,6}(?:-\d{3,4})?|\d{2}-\d{3}|[A-Z]{1,2}\d[A-Z\d]?[^\S\n]?\d[A-Z]{2})\b";
    let city = format!(
        r"{CAP}(?:[ \-](?:(?:{LINK}|sur|les)[ \-])*{CAP}){{0,3}}(?:[^\S\n]*,?[^\S\n]*(?:[A-Z]{{2}}\b|\([A-Z]{{2}}\)))?"
    );
    Regex::new(&format!(
        r"^(?:[^\S\n]*[,\-][^\S\n]*(?:{postcode}(?:[^\S\n]+{city})?|{city}(?:[^\S\n]*-?[^\S\n]*\d{{4,6}}(?:-\d{{3,4}})?)?(?:[^\S\n]*/[^\S\n]*[A-Z]{{2}}\b)?)|[^\S\n]+{postcode}(?:[^\S\n]+{city})?)"
    ))
    .expect("valid regex")
});

// Postcode alone (used to accept French type-only matches).
re!(
    POSTCODE,
    r"\b(?:\d{4,6}|\d{5}-\d{3}|\d{4}[^\S\n]?[A-Z]{2}|[A-Z]{1,2}\d[A-Z\d]?[^\S\n]?\d[A-Z]{2})\b"
);

// A unit after the street: ", Apt. 782", " Suite 140", ", Piano 3", ", lgh 1102".
re!(
    UNIT,
    r"^,?[^\S\n]*(?i:apt\.?|apartment|suite|ste\.?|unit|flat|piano|app\.?|appartamento|lgh\.?|lägenhet|whg\.?|wohnung|piso|puerta|bus|bte|studio)[^\S\n]*#?[^\S\n]*\d{1,5}[A-Za-z]?(?:/\d{1,4})?\b"
);

/// Extend `[start, end)` over a unit and up to three trailing postcode / city / state parts.
fn extend(text: &str, end: usize) -> usize {
    let e = extend_parts(text, end);
    // A full stop after a city name ends the sentence ("…, 91119, Motala."), unlike the one
    // of an abbreviation ("Main St.").
    let last = text[end..e].rsplit([' ', ',']).next().unwrap_or("");
    if e > end && last.ends_with('.') && last.chars().filter(|c| c.is_alphabetic()).count() > 3 {
        e - 1
    } else {
        e
    }
}

fn extend_parts(text: &str, mut end: usize) -> usize {
    if let Some(m) = UNIT.find(&text[end..]) {
        end += m.end();
    }
    for _ in 0..3 {
        let Some(m) = SEGMENT.find(&text[end..]) else {
            break;
        };
        let seg = m.as_str().trim_start_matches([',', ' ', '-', '\t']);
        if m.as_str().len() > 48 || seg.is_empty() {
            break;
        }
        // A segment must end at a token boundary ("68535019" is not "685350").
        if text[end + m.end()..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
        {
            break;
        }
        end += m.end();
    }
    end
}

/// Street suffixes that also end everyday words ("Clearing", "Factor", "Iceberg").
const AMBIGUOUS_SUFFIXES: &[&str] = &[
    "ring", "tor", "berg", "wall", "wal", "hof", "haven", "erf", "veld", "kamp", "markt", "hoek",
    "steig", "kade", "backe", "torg", "damm", "ufer", "graben", "stieg", "baan", "allé",
];

/// English words that end like a German/Dutch street suffix.
const NOT_STREETS: &[&str] = &[
    "spring",
    "string",
    "during",
    "boring",
    "offering",
    "keyring",
    "bring",
    "ring",
    "monitoring",
    "engineering",
    "substring",
    "tor",
    "mentor",
    "vector",
    "factor",
    "doctor",
    "sector",
    "editor",
    "monitor",
    "operator",
    "iterator",
    "generator",
    "connector",
    "detector",
    "actor",
    "constructor",
    "selector",
    "validator",
    "processor",
    "visitor",
    "author",
    "berg",
    "iceberg",
    "firewall",
    "paywall",
    "haven",
];

/// Street addresses in English, German, Dutch, French, Spanish, Italian, Portuguese,
/// Indian and Japanese formats.
pub struct AddressDetector;
impl Detector for AddressDetector {
    fn name(&self) -> &'static str {
        "address"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        let push = |out: &mut Vec<Finding>, s: usize, e: usize| {
            out.push(Finding {
                score: 0.85,
                ..Finding::new(s, extend(text, e), Kind::Address, "address")
            });
        };
        for m in EN.find_iter(text) {
            push(&mut out, m.start(), m.end());
        }
        for m in COMPOUND.find_iter(text) {
            // Number first ("7 Markboulevard"): suffixes that also end ordinary words
            // ("10 Clearing", "2 Factor") need a unit, postcode or city after the street.
            if m.as_str().starts_with(|c: char| c.is_ascii_digit()) {
                let word = m.as_str().rsplit(' ').next().unwrap_or("").to_lowercase();
                let plain = AMBIGUOUS_SUFFIXES.iter().any(|s| word.ends_with(s))
                    || NOT_STREETS.contains(&word.as_str());
                if plain && extend(text, m.end()) == m.end() {
                    continue;
                }
                push(&mut out, m.start(), m.end());
                continue;
            }
            let num: String = m
                .as_str()
                .chars()
                .rev()
                .take_while(|c| !c.is_whitespace())
                .collect();
            let num: String = num.chars().rev().filter(char::is_ascii_digit).collect();
            // "Spring 2024" style years are not house numbers; "String 3" is not a street.
            if num.len() == 4 && (num.starts_with("19") || num.starts_with("20")) {
                continue;
            }
            let word = m
                .as_str()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            if NOT_STREETS.contains(&word.as_str()) {
                continue;
            }
            // "Creditor 2", "Withdrawal 1": an ambiguous suffix on a single word needs a
            // postcode or city after the number.
            let single = m.as_str().split_whitespace().count() == 2;
            if single
                && AMBIGUOUS_SUFFIXES.iter().any(|s| word.ends_with(s))
                && extend(text, m.end()) == m.end()
            {
                continue;
            }
            push(&mut out, m.start(), m.end());
        }
        for m in FR.find_iter(text) {
            let numbered = m.as_str().starts_with(|c: char| c.is_ascii_digit());
            let end = extend(text, m.end());
            let has_postcode = POSTCODE.is_match(&text[m.end()..end]);
            if numbered || has_postcode {
                out.push(Finding {
                    score: 0.85,
                    ..Finding::new(m.start(), end, Kind::Address, "address")
                });
            }
        }
        for m in ROMANCE.find_iter(text).chain(ROMANCE_NUM.find_iter(text)) {
            push(&mut out, m.start(), m.end());
        }
        for m in IN.find_iter(text) {
            // "1,250.00, Street Address": the number is part of an amount.
            if text[..m.start()]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit() || c == '.' || c == ',')
            {
                continue;
            }
            let end = extend(text, m.end());
            let typed = m
                .as_str()
                .split_whitespace()
                .any(|w| IN_TYPES.contains(&w.trim_end_matches(',')));
            let pin = text[m.end()..end]
                .split(|c: char| !c.is_ascii_digit())
                .any(|d| d.len() == 6);
            if typed || pin {
                out.push(Finding {
                    score: 0.8,
                    ..Finding::new(m.start(), end, Kind::Address, "address")
                });
            }
        }
        for m in JP.find_iter(text) {
            out.push(Finding {
                score: 0.85,
                ..Finding::new(m.start(), m.end(), Kind::Address, "address")
            });
        }
        out
    }
}

// Legal forms that end an organisation name.
const ORG_LEGAL: &str = r"AB|HB|KB|SPA|Oyj|Oy|ApS|A/S|ASA|SRL|S\.R\.L\.?|S\.P\.A\.?|S\.L\.N\.E\.?|Sp\.[^\S\n]z[^\S\n]o\.o\.|s\.r\.o\.|BVBA|VZW|GmbH[^\S\n]&[^\S\n]Co\.[^\S\n]?KG|gGmbH|GmbH|GbR|PartG|mbB|AG|KGaA|KG|OHG|UG|e\.[^\S\n]?V\.|SE|Inc\.?|LLC|L\.L\.C\.|Ltd\.?|Limited|PLC|plc|LLP|Corp\.?|Corporation|Co\.,?[^\S\n]Ltd\.?|S\.A\.S\.?|SASU|SAS|SARL|S\.A\.R\.L\.|EURL|S\.A\.U?\.?|SA|S\.L\.U?\.?|SL|S\.Coop\.?|S\.p\.A\.?|SpA|S\.r\.l\.?|s\.r\.l\.?|Srl|S\.n\.c\.?|s\.n\.c\.?|S\.a\.s\.?|B\.V\.?|BV|N\.V\.?|NV|V\.O\.F\.?|Ltda\.?|S/A|EIRELI|Pvt\.?[^\S\n]Ltd\.?|Private[^\S\n]Limited|Pty\.?[^\S\n]Ltd\.?|&[^\S\n]Co\.?|&[^\S\n]Sons|and[^\S\n]Sons|&[^\S\n]Söhne|e[^\S\n]figli|e[^\S\n]Figli|e[^\S\n]Filhos|&[^\S\n]Filhos|e[^\S\n]Hijos|&[^\S\n]Hijos|Hermanos|y[^\S\n]Asociados|&[^\S\n]Asociados";

// A further legal form right after one ("Durand S.A.S. GmbH", "e figli Ltd.").
static ORG_TRAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^,?[^\S\n]+(?:{ORG_LEGAL})(?:[^\p{{L}}\p{{N}}]|$)"
    ))
    .expect("valid regex")
});

// Organisation = capitalised words + a legal form ("Acme GmbH", "Lamy S.A.S.", "Raj PLC").
static ORG: LazyLock<Regex> = LazyLock::new(|| {
    let legal = ORG_LEGAL;
    Regex::new(&format!(
        r"\b(?:{CAP}(?:,)?[^\S\n]+(?:(?:&|and|und|et|e|y|van|de|der|den|du|des|la|le|di|da|do|dos|das)[^\S\n]+)?){{1,4}}(?:{legal})(?:[^\p{{L}}\p{{N}}]|$)|[\p{{Han}}\p{{Katakana}}ー]{{1,12}}(?:株式会社|有限会社|合同会社)|(?:株式会社|有限会社|合同会社)[\p{{Han}}\p{{Katakana}}ー]{{1,12}}"
    ))
    .expect("valid regex")
});

const ORG_LEADING_NOISE: &[&str] = &[
    "The",
    "This",
    "These",
    "That",
    "Such",
    "Each",
    "Any",
    "All",
    "Every",
    "Their",
    "His",
    "Her",
    "Its",
    "Dieser",
    "Diese",
    "Dieses",
    "Ce",
    "Cette",
    "Este",
    "Esta",
    "Questo",
    "Questa",
    "Deze",
    "Dit",
    "Denna",
    "Detta",
    "A",
    "An",
    "Der",
    "Die",
    "Das",
    "Den",
    "Dem",
    "Des",
    "Ein",
    "Eine",
    "Le",
    "La",
    "Les",
    "El",
    "Los",
    "Il",
    "Lo",
    "De",
    "Het",
    "Een",
    "Un",
    "Une",
    "Una",
    "Unser",
    "Unsere",
    "Your",
    "Notre",
    "Nuestro",
    "Nuestra",
    "Nostra",
    "Nostro",
    "Vår",
    "Vårt",
    "Onze",
    "Our",
    "Email",
    "From",
    "With",
    "Contract",
    "Client",
    "Customer",
    "Vendor",
    "Supplier",
    "Partner",
    "Dear",
    "Hi",
    "Hello",
    "Thanks",
    "Please",
    "Invoice",
    "Ticket",
    "Re",
    "Fwd",
    "At",
    "For",
    "To",
    "And",
    "Summarise",
    "Summarize",
    "Compare",
    "Ask",
    "Call",
    "Write",
    "Send",
    "Tell",
    "Next",
    "Then",
    "Also",
    "However",
    "But",
    "So",
];

// A run of capitalised words with connectors ("Bank of the Pacific", "Fondo Agrícola Global").
static TITLE_RUN: LazyLock<Regex> = LazyLock::new(|| {
    let link =
        r"(?:of|and|&|for|de|del|della|di|du|des|van|der|den|für|y|e|och|und|et|the|la|le|los|las)";
    let word = r"(?:\p{Lu}[\p{L}'’\-]*\.?|\p{Lu}{2,6})";
    Regex::new(&format!(
        r"\b{word}(?:[^\S\n]+(?:{link}[^\S\n]+){{0,2}}{word}){{1,6}}"
    ))
    .expect("valid regex")
});

/// Nouns that head an organisation name ("… Insurance", "… Group", "Banco …", "Stichting …").
const ORG_HEADS: &[&str] = &[
    "Bank",
    "Banca",
    "Banco",
    "Banque",
    "Banken",
    "Bancorp",
    "Sparkasse",
    "Insurance",
    "Assurance",
    "Assurances",
    "Versicherung",
    "Versicherungen",
    "Verzekeringen",
    "Seguros",
    "Assicurazioni",
    "Försäkring",
    "Group",
    "Gruppe",
    "Groep",
    "Grupo",
    "Gruppo",
    "Groupe",
    "Gruppen",
    "Holding",
    "Holdings",
    "Capital",
    "Partners",
    "Fund",
    "Funds",
    "Fonds",
    "Fondo",
    "Foundation",
    "Stiftung",
    "Stichting",
    "Fundación",
    "Fondazione",
    "Fondation",
    "Stiftelse",
    "Trust",
    "Company",
    "Corporation",
    "Associates",
    "Investments",
    "Ventures",
    "Industries",
    "Enterprises",
    "Technologies",
    "Solutions",
    "Systems",
    "Consulting",
    "Agency",
    "Institute",
    "Institut",
    "Instituut",
    "University",
    "Universität",
    "Universidad",
    "Università",
    "Université",
    "Universiteit",
    "Universitet",
    "Hospital",
    "Clinic",
    "Klinik",
    "Kliniek",
    "Academy",
    "School",
    "College",
    "Healthcare",
    "Apotheke",
    "Lines",
    "Shipping",
    "Logistics",
    "Airlines",
    "Motors",
    "Pharmaceuticals",
    "Laboratories",
    "Labs",
    "Society",
    "Association",
    "Federation",
    "Advisors",
    "Brokers",
    "Securities",
    "Asset Management",
    "Realty",
    "Properties",
];

/// Common words of organisation and heading titles. A cue-word organisation needs one word
/// that is not among these (a surname, an invented brand, an acronym): "Torricelli Group",
/// "ABC Insurance Company", but not "Emergency Relief Fund" or "Investment Banking Group".
const GENERIC_ORG_WORDS: &[&str] = &[
    "The",
    "Global",
    "International",
    "National",
    "General",
    "First",
    "United",
    "American",
    "European",
    "Investment",
    "Investments",
    "Financial",
    "Finance",
    "Private",
    "Public",
    "Central",
    "Commercial",
    "Retail",
    "Corporate",
    "Customer",
    "Marketing",
    "Human",
    "Resources",
    "Health",
    "Risk",
    "Management",
    "Asset",
    "Assets",
    "Wealth",
    "Advisory",
    "Services",
    "Service",
    "Technology",
    "Digital",
    "Data",
    "Software",
    "Network",
    "Business",
    "Development",
    "Research",
    "Real",
    "Estate",
    "Property",
    "Life",
    "Mutual",
    "Savings",
    "Credit",
    "Loan",
    "Mortgage",
    "Equity",
    "Growth",
    "Income",
    "Value",
    "Strategic",
    "Sustainable",
    "Green",
    "Energy",
    "Innovation",
    "Innovations",
    "Banking",
    "Security",
    "Exchange",
    "Trading",
    "Market",
    "Markets",
    "Community",
    "Social",
    "Impact",
    "Education",
    "Medical",
    "Care",
    "Family",
    "Home",
    "Auto",
    "Travel",
    "Food",
    "Industrial",
    "Manufacturing",
    "Construction",
    "Transport",
    "Transportation",
    "Legal",
    "Compliance",
    "Audit",
    "Tax",
    "Accounting",
    "Operations",
    "Support",
    "Sales",
    "Team",
    "Department",
    "Division",
    "Office",
    "Board",
    "Committee",
    "Council",
    "Annual",
    "Quarterly",
    "Report",
    "Plan",
    "Program",
    "Programme",
    "Policy",
    "Agreement",
    "Overview",
    "Summary",
    "Introduction",
    "Statement",
    "Account",
    "Accounts",
    "Investor",
    "Investors",
    "Client",
    "Clients",
    "Employee",
    "Employees",
    "Benefits",
    "Pension",
    "Retirement",
    "Emergency",
    "Relief",
    "Charitable",
    "Charity",
    "New",
    "North",
    "South",
    "East",
    "West",
    "Northern",
    "Southern",
    "Eastern",
    "Western",
    "Your",
    "Our",
    "Their",
    "This",
    "That",
    "Name",
    "Global",
    "Royal",
    "Federal",
    "State",
    "City",
    "County",
    "Regional",
    "Local",
    "Internacional",
    "Nacional",
    "Europea",
    "Europäische",
    "Deutsche",
    "Allgemeine",
    "Nationale",
    "Generale",
    "Generali",
    "Italiana",
    "Española",
    "Svenska",
    "Nederlandse",
    "Française",
    "High",
    "Primary",
    "Secondary",
    "Senior",
    "Junior",
    "Medium",
    "Small",
    "Cap",
    "Mid",
    "Large",
    "Capital",
    "Holdings",
    "Fund",
    "Group",
    "Identifier",
    "Code",
    "Codes",
    "Information",
    "Info",
    "Details",
    "Detail",
    "Number",
    "Numbers",
    "Routing",
    "Transfer",
    "Transfers",
    "Wire",
    "Claim",
    "Claims",
    "Form",
    "Forms",
    "Rate",
    "Rates",
    "Strategy",
    "Updates",
    "Update",
    "Trends",
    "Industry",
    "Platform",
    "Transaction",
    "Transactions",
    "Operation",
    "Registration",
    "Conference",
    "Provider",
    "Providers",
    "Bond",
    "Bonds",
    "Venture",
    "Share",
    "Shares",
    "Liability",
    "Renters",
    "Title",
    "Interruption",
    "Disclosure",
    "Coverage",
    "Premium",
    "Product",
    "Products",
    "Portfolio",
    "Index",
    "Aggregate",
    "Performance",
    "Return",
    "Returns",
    "Detailed",
    "Grant",
    "Grants",
    "Funds",
    "Year",
    "Emerging",
    "Tech",
    "Award",
    "Awards",
    "Category",
    "Section",
    "Part",
    "Article",
    "Chapter",
    "Schedule",
    "Type",
    "Class",
    "Total",
    "Net",
    "Gross",
    "Cash",
    "Flow",
    "Balance",
    "Sheet",
    "Profit",
    "Loss",
    "Revenue",
    "Expenses",
    "Liabilities",
    "Notes",
    "Note",
    "Table",
    "Appendix",
    "Terms",
    "Conditions",
    "Specific",
    "Special",
    "Key",
    "Main",
    "Other",
    "Additional",
    "Personal",
    "Individual",
    "Joint",
    "Residential",
    "Dental",
    "Vision",
    "Disability",
    "Accident",
    "Casualty",
    "Motor",
    "Vehicle",
    "Pet",
    "Contents",
    "Flood",
    "Fire",
    "Earthquake",
    "Cyber",
    "Professional",
    "Indemnity",
    "Workers",
    "Compensation",
    "Unemployment",
    "Welfare",
    "Protection",
    "Future",
    "Power",
    "Bank",
    "Banking",
    "Insurance",
    "Company",
    "Corporation",
    "Trust",
    "Foundation",
    "School",
    "College",
    "Academy",
    "Hospital",
    "Clinic",
    "Agency",
    "Institute",
    "Lines",
    "Shipping",
    "Equipment",
    "Contract",
    "Contracts",
    "Claimant",
    "Policyholder",
    "Insured",
    "Option",
    "Options",
    "Request",
    "Application",
    "Letter",
    "Notice",
    "Confirmation",
    "Certificate",
    "Guarantee",
    "Payment",
    "Payments",
    "Fee",
    "Fees",
    "Charges",
    "Deposit",
    "Deposits",
    "Withdrawal",
    "Interest",
    "Dividend",
    "Dividends",
    "Tax",
    "Taxes",
    "Rules",
    "Regulations",
    "Guidelines",
    "Standards",
    "Requirements",
    "Procedures",
    "Process",
    "Review",
    "Analysis",
    "Assessment",
    "Evaluation",
    "Objectives",
    "Goals",
    "Mission",
    "Vision",
    "Values",
    "Principles",
    "Framework",
    "Model",
    "Structure",
    "Organization",
    "Organisation",
    "Governance",
    "Oversight",
    "Control",
    "Controls",
    "Internal",
    "External",
    "Independent",
    "Executive",
    "Chief",
    "Officer",
    "Director",
    "Directors",
    "Manager",
    "Members",
    "Member",
    "Partner",
    "Owner",
    "Holder",
    "Holders",
    "Beneficiary",
    "Agent",
    "Representative",
    "Contact",
    "Address",
    "Phone",
    "Email",
    "Website",
    "Online",
    "Mobile",
    "App",
    "Card",
    "Cards",
    "Debit",
    "Check",
    "Checking",
    "Current",
    "Fixed",
    "Variable",
    "Short",
    "Long",
    "Term",
    "Medium",
    "Annuity",
    "Insurers",
    "Brokerage",
    "Mortgages",
    "Loans",
    "Lenders",
    "Lender",
    "Borrower",
    "Borrowers",
    "Debt",
    "Bancaria",
    "Bancario",
    "Kasse",
    "Und",
    "Shaping",
    "Utilizing",
    "Institution",
    "Institutions",
    "Reference",
    "Resource",
    "Cryptocurrency",
    "Money",
    "Reverse",
    "Free",
    "Derivatives",
    "Whole",
    "Boat",
    "Umbrella",
    "Ransom",
    "Kidnap",
    "Universal",
    "Pet",
    "Homeowners",
    "Landlord",
    "Marine",
    "Cargo",
    "Crop",
    "Aviation",
    "Event",
    "Critical",
    "Illness",
    "Errors",
    "Omissions",
    "Malpractice",
    "Crime",
    "Surety",
    "Warranty",
    "Hurricane",
    "Storm",
    "Theft",
    "Glass",
    "Machinery",
    "Breakdown",
    "Excess",
    "Employers",
    "Employer's",
    "Officers",
    "Stock",
    "Total",
    "Diversified",
    "Balanced",
    "Bond",
    "Equity",
    "Income",
    "Dividend",
    "Treasury",
    "Municipal",
    "Sector",
    "Target",
    "Date",
    "Lifecycle",
];

/// Function words that open sentences in the supported languages ("Wenn Sie Ihre
/// Versicherung …"): capitalised, but never what makes a name distinctive.
const FUNCTION_WORDS: &[&str] = &[
    "When",
    "If",
    "We",
    "You",
    "They",
    "For",
    "With",
    "From",
    "At",
    "By",
    "To",
    "In",
    "On",
    "As",
    "Please",
    "Dear",
    "Wenn",
    "Sie",
    "Ihre",
    "Ihr",
    "Ihren",
    "Ihrem",
    "Wir",
    "Unser",
    "Unsere",
    "Bei",
    "Mit",
    "Für",
    "Von",
    "Vom",
    "Zur",
    "Zum",
    "Bitte",
    "Als",
    "Wie",
    "Auch",
    "Nach",
    "Bis",
    "Durch",
    "Ohne",
    "Über",
    "Unter",
    "Vor",
    "Alle",
    "Jede",
    "Jeder",
    "Kein",
    "Keine",
    "Wij",
    "Onze",
    "Uw",
    "Bij",
    "Met",
    "Voor",
    "Na",
    "Om",
    "Over",
    "Tot",
    "Uit",
    "Nous",
    "Vous",
    "Votre",
    "Notre",
    "Avec",
    "Pour",
    "Dans",
    "Sur",
    "Chez",
    "Par",
    "Usted",
    "Su",
    "Sus",
    "Con",
    "Para",
    "Por",
    "Sin",
    "Sobre",
    "Noi",
    "Voi",
    "Lei",
    "Suo",
    "Sua",
    "Per",
    "Vi",
    "Ni",
    "Er",
    "Din",
    "Ditt",
    "Med",
    "För",
    "Till",
    "Från",
    "Hos",
    "Sender",
    "Receiver",
    "Correspondent",
    "Issuing",
    "Acquiring",
    "Paying",
    "Receiving",
    "Homeowner",
    "Renter",
    "Tenant",
    "Owner",
    "Test",
    "Sample",
    "Example",
    "Demo",
    "Dummy",
];

/// Heads that start a name: "Banco Agrícola", "Stichting De Brug", "Bank of the Pacific".
const PREFIX_HEADS: &[&str] = &[
    "AB",
    "Bank",
    "Banca",
    "Banco",
    "Banque",
    "Stichting",
    "Stiftung",
    "Stiftelse",
    "Fondo",
    "Fondazione",
    "Fundación",
    "Fondation",
    "Fonds",
    "Universidad",
    "Università",
    "Université",
    "Universität",
    "University",
    "Universiteit",
    "Institut",
    "Instituto",
    "Istituto",
    "Gruppo",
    "Grupo",
    "Groupe",
    "Clinica",
    "Clínica",
    "Hospital",
    "Ospedale",
    "Academy",
    "Försäkringsbolaget",
    "Sparkasse",
    "Volksbank",
];

/// Acronyms that are not organisation names on their own.
const COMMON_ACRONYMS: &[&str] = &[
    "AI", "AP", "IT", "HR", "US", "UK", "EU", "UN", "PR", "QA", "ID", "TV", "OK", "CEO", "CFO",
    "CTO", "COO", "ESG", "IPO", "ETF", "REIT", "LLC", "USA", "FAQ", "PDF", "API", "VAT", "KYC",
    "AML", "GDPR", "SME", "B2B", "B2C", "CRM", "ERP", "IBAN", "BIC", "SWIFT", "PIN", "CVV",
];

/// A title word without trailing punctuation or possessive ("Group's" -> "Group").
fn bare(t: &str) -> &str {
    let t = t.trim_end_matches(['.', ',']);
    t.strip_suffix("'s")
        .or_else(|| t.strip_suffix("’s"))
        .unwrap_or(t)
}

/// Organisations named by a head noun rather than a legal form: "Torricelli Group",
/// "State Farm Insurance", "Banco Agrícola", "Stockholms Universitet".
fn cue_orgs(text: &str, out: &mut Vec<Finding>) {
    for m in TITLE_RUN.find_iter(text) {
        // Template slots and labels: "[Company Name]", "{Bank Code}", "Bank Routing Number:".
        let before = text[..m.start()].chars().next_back();
        let after = text[m.end()..].trim_start_matches(' ').chars().next();
        if matches!(before, Some('[' | '{' | '<')) || matches!(after, Some(':' | ']' | '}')) {
            continue;
        }
        let tokens: Vec<(usize, &str)> = m
            .as_str()
            .split([' ', '\t'])
            .scan(m.start(), |off, t| {
                let o = *off;
                *off += t.len() + 1;
                Some((o, t))
            })
            .filter(|(_, t)| !t.is_empty())
            .collect();
        // A full stop inside the run ends a sentence ("Response Time. ABC Logistics").
        let mut from = 0;
        for (i, (_, t)) in tokens.iter().enumerate() {
            let sentence_end = t.ends_with('.')
                && t.chars().filter(|c| c.is_alphabetic()).count() >= 3
                && !matches!(bare(t), "Inc" | "Corp" | "Ltd" | "Bros" | "Co");
            if sentence_end || i + 1 == tokens.len() {
                cue_org(text, &tokens[from..=i], out);
                from = i + 1;
            }
        }
    }
}

/// Words joining the parts of a name that starts with its head ("Bank of the Pacific").
const OF_LINKS: &[&str] = &[
    "of", "the", "de", "del", "della", "di", "du", "des", "van", "der", "den", "för", "für", "la",
    "le", "el", "los", "las",
];

/// True if `w` can carry an organisation name: a capitalised word that is not a generic
/// title word, a head, a common acronym or an adjective-like form ("Uncollected",
/// "Physical", "Intermediary", "Quality").
pub(crate) fn distinctive(w: &str) -> bool {
    let b = bare(w);
    let lower = b.to_lowercase();
    // English adjective and abstract-noun endings, and the noun endings of the other
    // languages ("Überweisung", "Sicherheit", "Gestione", "Inversión").
    const ADJ: &[&str] = &[
        "ed", "al", "ive", "ous", "ic", "ics", "ing", "ary", "ory", "ble", "ful", "less", "ment",
        "ments", "tion", "tions", "sion", "ance", "ence", "ity", "ism", "ist", "ure", "ness",
        "ship", "ung", "heit", "keit", "schaft", "tät", "heid", "ning", "zione", "ción", "sión",
        "miento", "dad", "ité",
    ];
    let roman = b.chars().all(|c| "IVXLC".contains(c));
    // "Short-term", "E-commerce": a compound with a lowercase part is a common word.
    let lower_compound = b
        .split('-')
        .skip(1)
        .any(|p| p.starts_with(char::is_lowercase));
    !roman
        && !lower_compound
        && b.starts_with(char::is_uppercase)
        && b.chars().filter(|c| c.is_alphabetic()).count() >= 2
        && !ORG_HEADS.contains(&b)
        && !GENERIC_ORG_WORDS.contains(&b)
        && !COMMON_ACRONYMS.contains(&b)
        && !FUNCTION_WORDS.contains(&b)
        && !super::names::is_org_word(b)
        && (b.chars().all(|c| !c.is_lowercase())
            || super::names::is_first_name(b)
            || !ADJ.iter().any(|s| lower.ends_with(s)))
}

/// Judge one sentence-bounded title run for a head-noun organisation.
fn cue_org(text: &str, tokens: &[(usize, &str)], out: &mut Vec<Finding>) {
    let first = tokens
        .iter()
        .position(|(_, t)| !ORG_LEADING_NOISE.contains(&bare(t)))
        .unwrap_or(tokens.len());
    let words = &tokens[first..];
    if words.len() < 2 {
        return;
    }
    let is_head = |t: &str| ORG_HEADS.contains(&bare(t));
    let capitalised = |t: &str| t.starts_with(char::is_uppercase) || t == "&";
    let mut span = None;
    // "Torricelli Group", "State Farm Insurance": capitalised words right before the head.
    if let Some(h) = words.iter().rposition(|(_, t)| is_head(t)) {
        let mut j = h;
        while j > 0 && capitalised(words[j - 1].1) && !ORG_LEADING_NOISE.contains(&words[j - 1].1) {
            j -= 1;
        }
        if h > j && words[j..h].iter().any(|(_, t)| distinctive(t)) {
            // "London School of Economics": the name goes on after "of".
            let mut k = h;
            while k + 2 < words.len()
                && OF_LINKS.contains(&words[k + 1].1)
                && capitalised(words[k + 2].1)
            {
                k += 2;
            }
            span = Some((j, k));
        }
    }
    // "Banco Agrícola", "Bank of the Pacific", "Stichting De Brug": a head that opens it.
    if span.is_none() && PREFIX_HEADS.contains(&bare(words[0].1)) {
        let mut k = 0;
        while k + 1 < words.len()
            && (capitalised(words[k + 1].1) || OF_LINKS.contains(&words[k + 1].1))
        {
            k += 1;
        }
        while k > 0 && OF_LINKS.contains(&words[k].1) {
            k -= 1;
        }
        if k > 0 && words[1..=k].iter().any(|(_, t)| distinctive(t)) {
            span = Some((0, k));
        }
    }
    if let Some((a, b)) = span {
        let (s, (lo, lt)) = (words[a].0, words[b]);
        let e = lo + bare(lt).len();
        // A kind of product, not a company: "a Whole Life Insurance policy",
        // "Policy Type: Term Insurance", "any Money Market Fund".
        let before = text[..s].trim_end_matches(' ').to_lowercase();
        let generic_article = [
            "a", "an", "any", "each", "every", "no", "un", "une", "ein", "eine", "una", "uno",
            "een", "en", "ett",
        ]
        .iter()
        .any(|w| before.ends_with(&format!(" {w}")) || before == *w);
        let after = text[e..].trim_start_matches(' ');
        const PRODUCT_NOUNS: &[&str] = &[
            "policy",
            "policies",
            "plan",
            "plans",
            "claim",
            "claims",
            "coverage",
            "premium",
            "product",
            "products",
            "account",
            "accounts",
            "contract",
            "agreement",
            "scheme",
            "option",
            "options",
            "units",
            "shares",
            "type",
            "types",
            "rate",
            "provider",
        ];
        let product = PRODUCT_NOUNS
            .iter()
            .any(|w| after.starts_with(w) && !after[w.len()..].starts_with(char::is_alphanumeric));
        let typed = {
            let line = before.rsplit('\n').next().unwrap_or("");
            line.contains("type") || line.contains("typ") || line.contains("tipo")
        };
        if generic_article || product || typed {
            return;
        }
        out.push(Finding {
            score: 0.75,
            ..Finding::new(s, e, Kind::Org, "org")
        });
    }
}

/// Organisations identified by a legal-form suffix (GmbH, Ltd, S.A., B.V., 株式会社, ...).
pub struct OrgDetector;
impl Detector for OrgDetector {
    fn name(&self) -> &'static str {
        "org"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        cue_orgs(text, &mut out);
        for m in ORG.find_iter(text) {
            let s = m.as_str();
            // The trailing boundary character is not part of the name.
            let mut e = s.len();
            if let Some(c) = s.chars().next_back() {
                if !c.is_alphanumeric() && c != '.' {
                    e -= c.len_utf8();
                }
            }
            let mut b = 0;
            let mut name = &s[..e];
            // A name doesn't cross a sentence end: "Globex Ltd. Next, Initech GmbH" is two
            // sentences, so start after the last ". " that still leaves a whole organisation.
            while let Some(i) = name.find(". ") {
                let rest = &name[i + 2..];
                if ORG.find(rest).is_none_or(|m| m.start() != 0) {
                    break;
                }
                b += i + 2;
                name = rest;
            }
            // Drop sentence-leading verbs and greeting words ("Contract with", "Email").
            while let Some((first, rest)) = name.split_once(char::is_whitespace) {
                if !ORG_LEADING_NOISE.contains(&first.trim_end_matches([',', ':'])) {
                    break;
                }
                let skip = name.len() - rest.trim_start().len();
                b += skip;
                name = &name[skip..];
            }
            let name = name.trim_end();
            let mut end = m.start() + b + name.len();
            // Stacked legal forms belong to the same name.
            while let Some(t) = ORG_TRAIL.find(&text[end..]) {
                let tail = t.as_str();
                let last = tail
                    .chars()
                    .next_back()
                    .filter(|c| !c.is_alphanumeric() && *c != '.');
                end += tail.len() - last.map_or(0, char::len_utf8);
            }
            if name.split_whitespace().count() >= 2 || name.chars().any(super::fields::is_cjk) {
                out.push(Finding {
                    score: 0.85,
                    ..Finding::new(m.start() + b, end, Kind::Org, "org")
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(d: &dyn Detector, t: &str) -> Vec<String> {
        d.detect(t)
            .iter()
            .map(|f| t[f.start..f.end].to_string())
            .collect()
    }

    #[test]
    fn addresses_across_locales() {
        let d = AddressDetector;
        let one = |t: &str| found(&d, t);
        assert_eq!(
            one("ship to 221B Baker Street, London, NW1 6XE please"),
            vec!["221B Baker Street, London, NW1 6XE"]
        );
        assert_eq!(
            one("lives at 0074 Oconnell Viaduct, South Deniseview, TX 38572."),
            vec!["0074 Oconnell Viaduct, South Deniseview, TX 38572"]
        );
        assert_eq!(
            one("an Hauptstraße 5, 10115 Berlin verschickt"),
            vec!["Hauptstraße 5, 10115 Berlin"]
        );
        assert_eq!(
            one("Zaralaan 93, 4679 KF, Brummen\nTel."),
            vec!["Zaralaan 93, 4679 KF, Brummen"]
        );
        assert_eq!(
            one("au 44, boulevard Martine Roche, 17989 Moreno-la-Forêt?"),
            vec!["44, boulevard Martine Roche, 17989 Moreno-la-Forêt"]
        );
        assert_eq!(
            one("en Calle Mayor 12, Madrid, 28013 y"),
            vec!["Calle Mayor 12, Madrid, 28013"]
        );
        assert_eq!(
            one("in Via Roma 10, 00184 Roma (RM)"),
            vec!["Via Roma 10, 00184 Roma (RM)"]
        );
        assert_eq!(
            one("Rua Augusta, 120, São Paulo"),
            vec!["Rua Augusta, 120, São Paulo"]
        );
        assert_eq!(
            one("at 69/94, Gulati Chowk, Coimbatore 077266"),
            vec!["69/94, Gulati Chowk, Coimbatore 077266"]
        );
        assert_eq!(
            one("住所は東京都千代田区丸の内1丁目9番2号です"),
            vec!["東京都千代田区丸の内1丁目9番2号"]
        );
        // Not addresses.
        assert!(one("I spent 3 Hours in the park and 2 days on Spring 2024.").is_empty());
        assert!(one("We ship Spring 6 and Rust 2021 to the team.").is_empty());
        assert!(one("Read the rue de la documentation").is_empty());
    }

    #[test]
    fn international_street_forms() {
        let d = AddressDetector;
        let one = |t: &str| found(&d, t);
        // Number first, with a German/Dutch/Swedish compound or a Romance street type.
        assert_eq!(
            one("con adress 7 Markboulevard, Apt. 782, har"),
            vec!["7 Markboulevard, Apt. 782"]
        );
        assert_eq!(
            one("en 33 Villagränd, 91119, Motala."),
            vec!["33 Villagränd, 91119, Motala"]
        );
        assert_eq!(
            one("Adress: Storgatan 4, 123 45 Stockholm"),
            vec!["Storgatan 4, 123 45 Stockholm"]
        );
        assert_eq!(
            one("of 83 Via Brancaccio, Apt. 25, 00153"),
            vec!["83 Via Brancaccio, Apt. 25, 00153"]
        );
        assert_eq!(
            one("at 67 Henry run, Apt. 3'"),
            vec!["67 Henry run, Apt. 3"]
        );
        assert_eq!(
            one("live at 151 Rotonda Osvaldo, Apt. 69. Before"),
            vec!["151 Rotonda Osvaldo, Apt. 69"]
        );
        // Suffixes that also end ordinary words need a postcode, city or unit.
        assert!(one("We sent 10 Clearing instructions.").is_empty());
        assert!(one("[Creditor 2]\n").is_empty());
        assert!(one("[Withdrawal 1]\n").is_empty());
        assert!(one("Amount: $1,250.00, Street Address: unknown").is_empty());
    }

    #[test]
    fn orgs_named_by_head_nouns() {
        let d = OrgDetector;
        let one = |t: &str| found(&d, t);
        assert_eq!(one("the Torricelli Group's home"), vec!["Torricelli Group"]);
        assert_eq!(
            one("insurance with State Farm Insurance. The"),
            vec!["State Farm Insurance"]
        );
        assert_eq!(one("from Banco Agrícola today"), vec!["Banco Agrícola"]);
        assert_eq!(
            one("Bank of the Blue Ridge\nStatement"),
            vec!["Bank of the Blue Ridge"]
        );
        assert_eq!(
            one("Fraktör: Nordic Shipping AB\n"),
            vec!["Nordic Shipping AB"]
        );
        // Headings, products, generic titles and sentences are not organisations.
        assert!(one("Bank Transfer Details").is_empty());
        assert!(one("Your policy is a Whole Life Insurance policy.").is_empty());
        assert!(one("II. Fund Objectives").is_empty());
        assert!(one("to the Emergency Relief Fund").is_empty());
        assert!(one("Wenn Sie Ihre Versicherung kündigen").is_empty());
        assert!(one("[Company Name]").is_empty());
    }

    #[test]
    fn stacked_legal_forms() {
        let t = "pour Durand S.A.S. GmbH : le virement";
        assert_eq!(found(&OrgDetector, t), vec!["Durand S.A.S. GmbH"]);
    }

    #[test]
    fn org_names_stop_at_sentence_ends() {
        let t = "We signed with Globex Ltd. Next, Initech GmbH & Co. KG agreed.";
        let f: Vec<&str> = OrgDetector
            .detect(t)
            .iter()
            .map(|f| &t[f.start..f.end])
            .collect();
        assert_eq!(f, vec!["Initech GmbH & Co. KG"]);
    }

    #[test]
    fn orgs_with_legal_form() {
        let d = OrgDetector;
        assert_eq!(
            found(&d, "Contract with Acme Widgets GmbH is signed."),
            vec!["Acme Widgets GmbH"]
        );
        assert_eq!(found(&d, "from Lamy S.A.S. today"), vec!["Lamy S.A.S."]);
        assert_eq!(found(&d, "山下農林有限会社の件"), vec!["山下農林有限会社"]);
        assert!(found(&d, "the AG said").is_empty());
        // A legal form glued inside a word does not start a new name.
        assert_eq!(found(&d, "with Otto OHG mbH SAS before"), vec!["Otto OHG"]);
    }
}
