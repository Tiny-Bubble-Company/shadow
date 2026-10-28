//! Personas: the persistent fake identity a machine presents to AI clouds.
//!
//! All generated values are ASCII so they survive JSON escaping and streaming untouched.

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Static data for one fake region.
pub struct Region {
    pub code: &'static str,
    pub country: &'static str,
    pub locale: &'static str,
    pub dial: &'static str,
    pub tld: &'static str,
    pub cities: &'static [(&'static str, &'static str)],
    pub first: &'static [&'static str],
    pub last: &'static [&'static str],
    pub stems: &'static [&'static str],
    pub company_suffix: &'static str,
    pub streets: &'static [&'static str],
}

macro_rules! s { ($($x:expr),* $(,)?) => { &[$($x),*] }; }

/// Built-in regions.
pub static REGIONS: &[Region] = &[
    Region {
        code: "CN",
        country: "China",
        locale: "zh-CN",
        dial: "86",
        tld: "cn",
        cities: s![
            ("Hangzhou", "Asia/Shanghai"),
            ("Shenzhen", "Asia/Shanghai"),
            ("Chengdu", "Asia/Shanghai"),
            ("Wuhan", "Asia/Shanghai")
        ],
        first: s![
            "Wei", "Fang", "Lei", "Jing", "Yong", "Xiu", "Ming", "Hui", "Jun", "Lan", "Tao", "Yan",
            "Bo", "Qiang", "Ying", "Hao"
        ],
        last: s![
            "Zhang", "Wang", "Li", "Liu", "Chen", "Yang", "Huang", "Zhao", "Zhou", "Wu", "Xu",
            "Sun", "Ma", "Zhu", "Hu", "Guo"
        ],
        stems: s![
            "Zhilan", "Huaxin", "Tianrui", "Jinyu", "Haoyun", "Lingxi", "Ruixin", "Yunfan",
            "Qiming", "Xinghe"
        ],
        company_suffix: "Technology Co., Ltd.",
        streets: s![
            "Wensan Road",
            "Jianguo Road",
            "Binjiang Avenue",
            "Keyuan Road"
        ],
    },
    Region {
        code: "JP",
        country: "Japan",
        locale: "ja-JP",
        dial: "81",
        tld: "jp",
        cities: s![
            ("Osaka", "Asia/Tokyo"),
            ("Fukuoka", "Asia/Tokyo"),
            ("Sapporo", "Asia/Tokyo"),
            ("Nagoya", "Asia/Tokyo")
        ],
        first: s![
            "Haruto", "Yui", "Sota", "Aoi", "Ren", "Hina", "Daiki", "Mio", "Kaito", "Sakura",
            "Riku", "Yuna", "Takumi", "Emi", "Kenji", "Nao"
        ],
        last: s![
            "Sato",
            "Suzuki",
            "Takahashi",
            "Tanaka",
            "Watanabe",
            "Ito",
            "Yamamoto",
            "Nakamura",
            "Kobayashi",
            "Kato",
            "Yoshida",
            "Yamada",
            "Sasaki",
            "Matsumoto",
            "Inoue",
            "Kimura"
        ],
        stems: s![
            "Kazetech", "Hoshiba", "Mirai", "Asahina", "Kitsune", "Tsubame", "Yamabuki", "Hikari",
            "Kurenai", "Sorano"
        ],
        company_suffix: "K.K.",
        streets: s!["Midosuji", "Tenjin Dori", "Odori Nishi", "Sakae Chome"],
    },
    Region {
        code: "KR",
        country: "South Korea",
        locale: "ko-KR",
        dial: "82",
        tld: "kr",
        cities: s![
            ("Busan", "Asia/Seoul"),
            ("Daegu", "Asia/Seoul"),
            ("Incheon", "Asia/Seoul")
        ],
        first: s![
            "Minjun", "Seoyeon", "Jiho", "Haeun", "Dohyun", "Jiwoo", "Yejun", "Sua", "Hyunwoo",
            "Chaewon", "Jimin", "Eunji"
        ],
        last: s![
            "Kim", "Lee", "Park", "Choi", "Jung", "Kang", "Cho", "Yoon", "Jang", "Lim", "Han",
            "Shin"
        ],
        stems: s!["Hanbit", "Saebom", "Mirae", "Daon", "Haneul", "Bitna", "Gaon", "Nuri"],
        company_suffix: "Co., Ltd.",
        streets: s!["Haeundae-ro", "Dongseong-ro", "Songdo-daero"],
    },
    Region {
        code: "BR",
        country: "Brazil",
        locale: "pt-BR",
        dial: "55",
        tld: "com.br",
        cities: s![
            ("Curitiba", "America/Sao_Paulo"),
            ("Recife", "America/Recife"),
            ("Porto Alegre", "America/Sao_Paulo"),
            ("Belo Horizonte", "America/Sao_Paulo")
        ],
        first: s![
            "Joao", "Maria", "Pedro", "Ana", "Lucas", "Juliana", "Gabriel", "Beatriz", "Rafael",
            "Larissa", "Thiago", "Camila", "Bruno", "Leticia", "Diego", "Fernanda"
        ],
        last: s![
            "Silva",
            "Santos",
            "Oliveira",
            "Souza",
            "Rodrigues",
            "Ferreira",
            "Alves",
            "Pereira",
            "Lima",
            "Gomes",
            "Ribeiro",
            "Carvalho",
            "Almeida",
            "Lopes",
            "Barbosa",
            "Rocha"
        ],
        stems: s![
            "Araucaria",
            "Ipanema",
            "Tucano",
            "Jacaranda",
            "Pitanga",
            "Aurora",
            "Capivara",
            "Maracuja"
        ],
        company_suffix: "Tecnologia Ltda.",
        streets: s![
            "Rua XV de Novembro",
            "Avenida Boa Viagem",
            "Rua dos Andradas"
        ],
    },
    Region {
        code: "US",
        country: "United States",
        locale: "en-US",
        dial: "1",
        tld: "com",
        cities: s![
            ("Denver", "America/Denver"),
            ("Austin", "America/Chicago"),
            ("Portland", "America/Los_Angeles"),
            ("Raleigh", "America/New_York")
        ],
        first: s![
            "James", "Olivia", "Ethan", "Emma", "Mason", "Ava", "Logan", "Mia", "Carter", "Harper",
            "Wyatt", "Ella", "Hunter", "Avery", "Grant", "Riley"
        ],
        last: s![
            "Whitaker", "Calloway", "Brennan", "Hollis", "Prescott", "Langford", "Mercer",
            "Thornton", "Delaney", "Ashford", "Kendrick", "Sutton", "Garrison", "Holloway",
            "Pruitt", "Rowland"
        ],
        stems: s![
            "Bluepine",
            "Ironwood",
            "Cedarline",
            "Summitry",
            "Northgate",
            "Clearbrook",
            "Redrock",
            "Harborview"
        ],
        company_suffix: "Inc.",
        streets: s![
            "Larimer Street",
            "Congress Avenue",
            "Burnside Street",
            "Glenwood Avenue"
        ],
    },
    Region {
        code: "DE",
        country: "Germany",
        locale: "de-DE",
        dial: "49",
        tld: "de",
        cities: s![
            ("Leipzig", "Europe/Berlin"),
            ("Bremen", "Europe/Berlin"),
            ("Dresden", "Europe/Berlin")
        ],
        first: s![
            "Lukas", "Lena", "Jonas", "Hannah", "Felix", "Leonie", "Paul", "Marie", "Elias",
            "Johanna", "Moritz", "Clara"
        ],
        last: s![
            "Brandt", "Kessler", "Vogt", "Lindner", "Hartmann", "Krause", "Seidel", "Winkler",
            "Arnold", "Fuchs", "Lorenz", "Haas"
        ],
        stems: s![
            "Nordlicht",
            "Elbwerk",
            "Sternwarte",
            "Lindenhof",
            "Waldkern",
            "Rheinblick"
        ],
        company_suffix: "GmbH",
        streets: s!["Karl-Liebknecht-Strasse", "Obernstrasse", "Prager Strasse"],
    },
    Region {
        code: "IN",
        country: "India",
        locale: "en-IN",
        dial: "91",
        tld: "in",
        cities: s![
            ("Pune", "Asia/Kolkata"),
            ("Jaipur", "Asia/Kolkata"),
            ("Kochi", "Asia/Kolkata"),
            ("Indore", "Asia/Kolkata")
        ],
        first: s![
            "Aarav", "Diya", "Vihaan", "Ananya", "Arjun", "Ishita", "Kabir", "Meera", "Rohan",
            "Saanvi", "Aditya", "Kavya"
        ],
        last: s![
            "Sharma", "Verma", "Iyer", "Nair", "Reddy", "Kulkarni", "Desai", "Mehta", "Joshi",
            "Bhat", "Chauhan", "Pillai"
        ],
        stems: s![
            "Navyuga",
            "Suryoday",
            "Akashdeep",
            "Tarang",
            "Prerna",
            "Vistaar"
        ],
        company_suffix: "Pvt. Ltd.",
        streets: s!["FC Road", "MI Road", "MG Road"],
    },
    Region {
        code: "AU",
        country: "Australia",
        locale: "en-AU",
        dial: "61",
        tld: "com.au",
        cities: s![
            ("Adelaide", "Australia/Adelaide"),
            ("Perth", "Australia/Perth"),
            ("Brisbane", "Australia/Brisbane"),
            ("Hobart", "Australia/Hobart")
        ],
        first: s![
            "Jack",
            "Charlotte",
            "Oliver",
            "Isla",
            "Noah",
            "Matilda",
            "Lachlan",
            "Ruby",
            "Cooper",
            "Zara",
            "Hamish",
            "Tahlia"
        ],
        last: s![
            "Mitchell", "Kelly", "Harrison", "Murray", "Fraser", "Dawson", "Ellis", "Gibson",
            "Hayes", "Kerr", "Lawson", "Ryan"
        ],
        stems: s![
            "Wattle",
            "Southcross",
            "Coralsea",
            "Bluegum",
            "Kookaburra",
            "Outback"
        ],
        company_suffix: "Pty Ltd",
        streets: s!["Rundle Street", "Hay Street", "Queen Street"],
    },
    Region {
        code: "MX",
        country: "Mexico",
        locale: "es-MX",
        dial: "52",
        tld: "com.mx",
        cities: s![
            ("Guadalajara", "America/Mexico_City"),
            ("Monterrey", "America/Monterrey"),
            ("Puebla", "America/Mexico_City")
        ],
        first: s![
            "Santiago",
            "Valentina",
            "Mateo",
            "Ximena",
            "Emiliano",
            "Regina",
            "Leonardo",
            "Renata",
            "Diego",
            "Fernanda",
            "Sebastian",
            "Daniela"
        ],
        last: s![
            "Hernandez",
            "Garcia",
            "Martinez",
            "Lopez",
            "Gonzalez",
            "Perez",
            "Sanchez",
            "Ramirez",
            "Cruz",
            "Flores",
            "Morales",
            "Reyes"
        ],
        stems: s![
            "Quetzal",
            "Tlaloc",
            "Agave",
            "Colibri",
            "Sierra Alta",
            "Nopal"
        ],
        company_suffix: "S.A. de C.V.",
        streets: s!["Avenida Chapultepec", "Calle Morelos", "Paseo Montejo"],
    },
    Region {
        code: "ZA",
        country: "South Africa",
        locale: "en-ZA",
        dial: "27",
        tld: "co.za",
        cities: s![
            ("Durban", "Africa/Johannesburg"),
            ("Pretoria", "Africa/Johannesburg"),
            ("Gqeberha", "Africa/Johannesburg")
        ],
        first: s![
            "Thabo", "Naledi", "Sipho", "Lerato", "Kagiso", "Zanele", "Bongani", "Amahle",
            "Mandla", "Palesa"
        ],
        last: s![
            "Nkosi", "Dlamini", "Ndlovu", "Mokoena", "Khumalo", "Mahlangu", "Botha", "Pillay",
            "Naidoo", "Molefe"
        ],
        stems: s!["Ubuntu", "Protea", "Baobab", "Karoo", "Indaba", "Lekker"],
        company_suffix: "(Pty) Ltd",
        streets: s!["Florida Road", "Church Street", "Govan Mbeki Avenue"],
    },
    Region {
        code: "CA",
        country: "Canada",
        locale: "en-CA",
        dial: "1",
        tld: "ca",
        cities: s![
            ("Halifax", "America/Halifax"),
            ("Winnipeg", "America/Winnipeg"),
            ("Calgary", "America/Edmonton")
        ],
        first: s![
            "Liam", "Chloe", "Owen", "Sophie", "Nathan", "Avery", "Felix", "Leah", "Rowan", "Maeve"
        ],
        last: s![
            "Tremblay", "Gagnon", "Roy", "Bouchard", "Gauthier", "Morin", "Lavoie", "Fortin",
            "Gagne", "Ouellet"
        ],
        stems: s![
            "Maplecrest",
            "Northwind",
            "Laurentide",
            "Aurora Bay",
            "Caribou"
        ],
        company_suffix: "Ltd.",
        streets: s!["Barrington Street", "Portage Avenue", "Stephen Avenue"],
    },
];

/// Look up a region by ISO code or country name (case-insensitive).
pub fn region(code: &str) -> Option<&'static Region> {
    let c = code.trim();
    REGIONS
        .iter()
        .find(|r| r.code.eq_ignore_ascii_case(c) || r.country.eq_ignore_ascii_case(c))
}

/// Pick a region far from the machine's real timezone.
pub fn far_region(real_tz: &str, rng: &mut impl Rng) -> &'static Region {
    let pool: &[&str] = match real_tz.split('/').next().unwrap_or("") {
        "Europe" | "Africa" | "Atlantic" => &["CN", "JP", "KR", "BR", "AU", "MX"],
        "America" | "US" | "Canada" => &["CN", "JP", "KR", "IN", "AU"],
        "Asia" | "Indian" => &["BR", "US", "MX", "CA", "ZA"],
        "Australia" | "Pacific" => &["DE", "BR", "US", "CA", "ZA"],
        _ => &["CN"],
    };
    region(pool.choose(rng).copied().unwrap_or("CN")).expect("built-in region")
}

/// The machine's fake identity. Persisted as `persona.json`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Persona {
    pub region: String,
    pub country: String,
    pub city: String,
    pub timezone: String,
    pub locale: String,
    pub dial_code: String,
    pub company: String,
    /// Company name without legal suffix (used for bare org mentions).
    pub company_short: String,
    pub domain: String,
    /// Primary person (stands in for the machine's real user).
    pub full_name: String,
    pub email: String,
    pub username: String,
    pub user_id: String,
    pub hostname: String,
    /// A few extra people of the persona's organisation.
    pub people: Vec<String>,
    pub created_at: String,
}

fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "-")
}

impl Persona {
    /// Generate a new persona for `region` (or a far-away region when `None`/`auto`).
    pub fn generate(region_code: Option<&str>, rng: &mut impl Rng) -> Persona {
        let reg = match region_code.filter(|r| !r.eq_ignore_ascii_case("auto")) {
            Some(c) => region(c).unwrap_or_else(|| {
                tracing::warn!("unknown region {c:?}; falling back to automatic choice");
                far_region(&local_timezone(), rng)
            }),
            None => far_region(&local_timezone(), rng),
        };
        let (city, tz) = *reg.cities.choose(rng).unwrap();
        let stem = *reg.stems.choose(rng).unwrap();
        let company = format!("{stem} {}", reg.company_suffix);
        let domain = format!("{}.{}", slug(stem), reg.tld);
        let first = *reg.first.choose(rng).unwrap();
        let last = *reg.last.choose(rng).unwrap();
        let full_name = format!("{first} {last}");
        let username = format!(
            "{}{}",
            first.to_ascii_lowercase(),
            last.to_ascii_lowercase()
        );
        let mut people = Vec::new();
        while people.len() < 4 {
            let p = format!(
                "{} {}",
                reg.first.choose(rng).unwrap(),
                reg.last.choose(rng).unwrap()
            );
            if p != full_name && !people.contains(&p) {
                people.push(p);
            }
        }
        Persona {
            region: reg.code.into(),
            country: reg.country.into(),
            city: city.into(),
            timezone: tz.into(),
            locale: reg.locale.into(),
            dial_code: reg.dial.into(),
            company,
            company_short: stem.into(),
            email: format!(
                "{}.{}@{}",
                first.to_ascii_lowercase(),
                last.to_ascii_lowercase(),
                domain
            ),
            domain,
            full_name,
            user_id: format!("user-{:012x}", rng.gen::<u64>() & 0xffff_ffff_ffff),
            hostname: format!(
                "{}-{}-{:02}",
                slug(stem),
                ["ws", "dev", "lt", "srv"].choose(rng).unwrap(),
                rng.gen_range(1..99)
            ),
            username,
            people,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Static data of the persona's region (falls back to CN).
    pub fn region_data(&self) -> &'static Region {
        region(&self.region).unwrap_or(&REGIONS[0])
    }

    /// Apply company/domain overrides from config.
    pub fn apply_overrides(&mut self, cfg: &crate::config::PersonaConfig) {
        if let Some(c) = &cfg.company {
            self.company = c.clone();
            self.company_short = crate::detect::dictionary::strip_legal_suffix(c).to_string();
        }
        if let Some(d) = &cfg.domain {
            self.domain = d.clone();
            if let Some((local, _)) = self.email.split_once('@') {
                self.email = format!("{local}@{d}");
            }
        }
    }
}

/// Best-effort IANA timezone of this machine.
pub fn local_timezone() -> String {
    std::env::var("TZ")
        .ok()
        .filter(|t| t.contains('/'))
        .or_else(|| iana_time_zone::get_timezone().ok())
        .unwrap_or_default()
}

const SYL_A: &[&str] = &[
    "vel", "kor", "lum", "tav", "zen", "mar", "quil", "dra", "sol", "nex", "or", "tal", "ver",
    "bry", "cas", "fen",
];
const SYL_B: &[&str] = &[
    "ora", "ix", "en", "ari", "ond", "eth", "ium", "ova", "ane", "yx", "ula", "ent", "aro", "ine",
];

/// A pronounceable capitalised codeword such as `Velora` or `Quilent`.
pub fn codeword(rng: &mut impl Rng) -> String {
    let mut w = String::new();
    w.push_str(SYL_A.choose(rng).unwrap());
    if rng.gen_bool(0.4) {
        w.push_str(SYL_A.choose(rng).unwrap());
    }
    w.push_str(SYL_B.choose(rng).unwrap());
    let mut c = w.chars();
    let first = c.next().unwrap().to_ascii_uppercase();
    std::iter::once(first).chain(c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_ascii_persona_in_requested_region() {
        let mut rng = rand::thread_rng();
        let p = Persona::generate(Some("cn"), &mut rng);
        assert_eq!(p.region, "CN");
        assert_eq!(p.timezone, "Asia/Shanghai");
        assert!(p.domain.ends_with(".cn"));
        for s in [&p.company, &p.full_name, &p.email, &p.hostname, &p.user_id] {
            assert!(s.is_ascii());
        }
    }

    #[test]
    fn far_region_is_far() {
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let r = far_region("Europe/Berlin", &mut rng);
            assert!(!matches!(r.code, "DE" | "ZA"));
        }
    }
}
