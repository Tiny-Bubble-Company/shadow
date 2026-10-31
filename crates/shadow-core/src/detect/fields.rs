//! Context-driven detectors: values whose *position* says what they are.
//!
//! - [`BirthDateDetector`]: dates right after a birth keyword ("born", "DOB", "Geburtsdatum",
//!   "né le", "fecha de nacimiento", ...).
//! - [`FieldDetector`]: `Label: value` lines ("Name:", "Address:", "Tel:", "Company:"), CSV
//!   columns identified by their header, and the name line of an email signature (`--`).
//!
//! Every value is still shape-checked (a phone column must hold a phone-like value, a date
//! field a valid date), so mislabelled or free-text fields are left alone.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::address::distinctive;
use super::dictionary::is_org_name;
use super::names::{
    double_barrelled, is_first_name, is_initial, is_org_word, looks_like_name, PARTICLES,
};
use super::patterns::{dummy_number, phone_shape};
use super::{Detector, Finding, Kind};
use crate::config::DetectorToggles;

/// Month names (full, then common abbreviations) per language. Index = month - 1.
pub(crate) const MONTHS: &[[&str; 12]] = &[
    [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ],
    [
        "januar",
        "februar",
        "märz",
        "april",
        "mai",
        "juni",
        "juli",
        "august",
        "september",
        "oktober",
        "november",
        "dezember",
    ],
    [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    [
        "januari",
        "februari",
        "maart",
        "april",
        "mei",
        "juni",
        "juli",
        "augustus",
        "september",
        "oktober",
        "november",
        "december",
    ],
    [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ],
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ],
];

/// `(language row, month index 0..12)` for a month name or abbreviation.
pub(crate) fn month_lookup(word: &str) -> Option<(usize, usize)> {
    let w = word.trim_end_matches('.').to_lowercase();
    let w = if w == "sept" { "sep".to_string() } else { w };
    MONTHS
        .iter()
        .enumerate()
        .find_map(|(row, names)| names.iter().position(|n| *n == w).map(|i| (row, i)))
}

static DATE: LazyLock<Regex> = LazyLock::new(|| {
    let mut names: Vec<&str> = MONTHS.iter().flatten().copied().collect();
    names.push("sept");
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    names.dedup();
    let m = names.join("|");
    Regex::new(&format!(
        r"(?i)\b(?:\d{{4}}[-./]\d{{1,2}}[-./]\d{{1,2}}|\d{{1,2}}[-./]\d{{1,2}}[-./](?:\d{{4}}|\d{{2}})|(?:{m})\.?\s+\d{{1,2}}(?:st|nd|rd|th)?,?\s+\d{{4}}|\d{{1,2}}(?:st|nd|rd|th|er|º|°)?\.?\s+(?:de\s+|of\s+)?(?:{m})\.?,?\s+(?:de\s+)?\d{{4}})\b"
    ))
    .expect("valid regex")
});

/// Numeric parts of a date: `(year, month, day)` if the text is a plausible calendar date.
pub(crate) fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let nums: Vec<&str> = s
        .split(|c: char| !c.is_ascii_digit())
        .filter(|t| !t.is_empty())
        .collect();
    let month_word = s
        .split(|c: char| !c.is_alphabetic())
        .find_map(month_lookup)
        .map(|(_, i)| i as u32 + 1);
    let (y, m, d) = match (month_word, nums.as_slice()) {
        (Some(m), [a, b]) if a.len() == 4 => (a.parse().ok()?, m, b.parse().ok()?),
        (Some(m), [a, b]) => (b.parse().ok()?, m, a.parse().ok()?),
        (None, [a, b, c]) if a.len() == 4 => (a.parse().ok()?, b.parse().ok()?, c.parse().ok()?),
        (None, [a, b, c]) => {
            let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
            let y: i32 = c.parse().ok()?;
            let y = if c.len() == 2 { 1900 + y } else { y };
            if a > 12 {
                (y, b, a)
            } else {
                (y, a, b)
            }
        }
        _ => return None,
    };
    let y = if y < 100 { 1900 + y } else { y };
    ((1..=12).contains(&m) && (1..=31).contains(&d) && (1880..=2100).contains(&y))
        .then_some((y, m, d))
}

/// The first date at the start of `s` (after at most a few connecting words).
fn date_after(s: &str) -> Option<(usize, usize)> {
    let line_end = s.find('\n').unwrap_or(s.len());
    let m = DATE.find(&s[..line_end])?;
    let gap = &s[..m.start()];
    let words = gap
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphabetic));
    (m.start() <= 24 && !gap.chars().any(|c| c.is_ascii_digit()) && words.count() <= 2)
        .then_some((m.start(), m.end()))
        .filter(|(a, b)| parse_date(&s[*a..*b]).is_some())
}

static BIRTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:born|date\s+of\s+birth|birth\s*date|birthday|d\.?o\.?b\b\.?|geboren|geburtsdatum|geburtstag|geb\.|née?\s+le|date\s+de\s+naissance|fecha\s+de\s+nacimiento|nacid[oa]|data\s+di\s+nascita|nat[oa]\s+il|geboortedatum|data\s+de\s+nascimento|nascid[oa])",
    )
    .expect("valid regex")
});

/// Dates of birth: a date shortly after a birth keyword, in several languages.
pub struct BirthDateDetector;
impl Detector for BirthDateDetector {
    fn name(&self) -> &'static str {
        "date"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        for k in BIRTH.find_iter(text) {
            if let Some((a, b)) = date_after(&text[k.end()..]) {
                out.push(Finding::new(k.end() + a, k.end() + b, Kind::Date, "date"));
            }
        }
        out
    }
}

/// What a labelled field or CSV column holds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum FieldKind {
    /// A person, where the label alone is strong evidence (customer, patient, full name).
    Person,
    /// A generic "name" label: needs a name-like value with a known first name.
    Name,
    /// "<something> name": a person only when the value has a given name or an initial.
    NameOnly,
    /// A party that may be a person or an organisation (payee, beneficiary, signatory):
    /// decided by the value's shape.
    Party,
    Address,
    Phone,
    Date,
    Org,
    Username,
    NationalId,
}

const FIELD_LABELS: &[(&str, FieldKind)] = &[
    ("name", FieldKind::Name),
    ("nom", FieldKind::Name),
    ("nombre", FieldKind::Name),
    ("nome", FieldKind::Name),
    ("naam", FieldKind::Name),
    ("full name", FieldKind::Person),
    ("fullname", FieldKind::Person),
    ("customer", FieldKind::Person),
    ("customer name", FieldKind::Person),
    ("client", FieldKind::Person),
    ("client name", FieldKind::Person),
    ("contact", FieldKind::Person),
    ("contact name", FieldKind::Person),
    ("contact person", FieldKind::Person),
    ("emergency contact", FieldKind::Person),
    ("employee", FieldKind::Person),
    ("employee name", FieldKind::Person),
    ("patient", FieldKind::Person),
    ("patient name", FieldKind::Person),
    ("applicant", FieldKind::Person),
    ("candidate", FieldKind::Person),
    ("tenant", FieldKind::Person),
    ("guest", FieldKind::Person),
    ("passenger", FieldKind::Person),
    ("beneficiary", FieldKind::Person),
    ("account holder", FieldKind::Person),
    ("cardholder", FieldKind::Person),
    ("card holder", FieldKind::Person),
    ("recipient", FieldKind::Person),
    ("first name", FieldKind::Person),
    ("last name", FieldKind::Person),
    ("surname", FieldKind::Person),
    ("given name", FieldKind::Person),
    ("family name", FieldKind::Person),
    ("nombre completo", FieldKind::Person),
    ("apellido", FieldKind::Person),
    ("apellidos", FieldKind::Person),
    ("nom complet", FieldKind::Person),
    ("prénom", FieldKind::Person),
    ("nom de famille", FieldKind::Person),
    ("vorname", FieldKind::Person),
    ("nachname", FieldKind::Person),
    ("familienname", FieldKind::Person),
    ("voornaam", FieldKind::Person),
    ("achternaam", FieldKind::Person),
    ("cognome", FieldKind::Person),
    ("nome completo", FieldKind::Person),
    ("sobrenome", FieldKind::Person),
    ("氏名", FieldKind::Person),
    ("名前", FieldKind::Person),
    ("address", FieldKind::Address),
    ("home address", FieldKind::Address),
    ("street address", FieldKind::Address),
    ("mailing address", FieldKind::Address),
    ("postal address", FieldKind::Address),
    ("billing address", FieldKind::Address),
    ("shipping address", FieldKind::Address),
    ("delivery address", FieldKind::Address),
    ("residential address", FieldKind::Address),
    ("adresse", FieldKind::Address),
    ("anschrift", FieldKind::Address),
    ("wohnadresse", FieldKind::Address),
    ("dirección", FieldKind::Address),
    ("direccion", FieldKind::Address),
    ("domicilio", FieldKind::Address),
    ("indirizzo", FieldKind::Address),
    ("residenza", FieldKind::Address),
    ("adres", FieldKind::Address),
    ("woonadres", FieldKind::Address),
    ("endereço", FieldKind::Address),
    ("endereco", FieldKind::Address),
    ("morada", FieldKind::Address),
    ("住所", FieldKind::Address),
    ("phone", FieldKind::Phone),
    ("phone number", FieldKind::Phone),
    ("telephone", FieldKind::Phone),
    ("tel", FieldKind::Phone),
    ("mobile", FieldKind::Phone),
    ("mobile phone", FieldKind::Phone),
    ("mobile number", FieldKind::Phone),
    ("cell", FieldKind::Phone),
    ("cell phone", FieldKind::Phone),
    ("fax", FieldKind::Phone),
    ("work phone", FieldKind::Phone),
    ("home phone", FieldKind::Phone),
    ("telefon", FieldKind::Phone),
    ("telefonnummer", FieldKind::Phone),
    ("handy", FieldKind::Phone),
    ("mobil", FieldKind::Phone),
    ("téléphone", FieldKind::Phone),
    ("tél", FieldKind::Phone),
    ("portable", FieldKind::Phone),
    ("teléfono", FieldKind::Phone),
    ("móvil", FieldKind::Phone),
    ("telefono", FieldKind::Phone),
    ("cellulare", FieldKind::Phone),
    ("telefoon", FieldKind::Phone),
    ("mobiel", FieldKind::Phone),
    ("telefone", FieldKind::Phone),
    ("celular", FieldKind::Phone),
    ("電話", FieldKind::Phone),
    ("電話番号", FieldKind::Phone),
    ("date of birth", FieldKind::Date),
    ("dob", FieldKind::Date),
    ("d.o.b", FieldKind::Date),
    ("birth date", FieldKind::Date),
    ("birthdate", FieldKind::Date),
    ("birthday", FieldKind::Date),
    ("geburtsdatum", FieldKind::Date),
    ("date de naissance", FieldKind::Date),
    ("fecha de nacimiento", FieldKind::Date),
    ("data di nascita", FieldKind::Date),
    ("geboortedatum", FieldKind::Date),
    ("data de nascimento", FieldKind::Date),
    ("生年月日", FieldKind::Date),
    ("company", FieldKind::Org),
    ("company name", FieldKind::Org),
    ("organisation", FieldKind::Org),
    ("organization", FieldKind::Org),
    ("employer", FieldKind::Org),
    ("firma", FieldKind::Org),
    ("unternehmen", FieldKind::Org),
    ("arbeitgeber", FieldKind::Org),
    ("société", FieldKind::Org),
    ("entreprise", FieldKind::Org),
    ("employeur", FieldKind::Org),
    ("empresa", FieldKind::Org),
    ("azienda", FieldKind::Org),
    ("società", FieldKind::Org),
    ("bedrijf", FieldKind::Org),
    ("werkgever", FieldKind::Org),
    ("username", FieldKind::Username),
    ("user name", FieldKind::Username),
    ("login", FieldKind::Username),
    ("ssn", FieldKind::NationalId),
    ("national id", FieldKind::NationalId),
    ("passport", FieldKind::NationalId),
    ("passport number", FieldKind::NationalId),
];

/// More labels, by language (normalised: lowercase, `_`/`-` as spaces). Pipe-separated
/// per kind to keep the table readable.
const MORE_LABELS: &[(&str, FieldKind)] = &[
    (
        "name of the insured|insured name|name of insured|policyholder name|policy holder name|\
         account holder name|cardholder name|card holder name|name on card|name on account|\
         holder name|owner name|beneficiary name|payee name|payer name|sender name|\
         recipient name|receiver name|debtor name|creditor name|investor name|borrower name|\
         applicant name|employee name|insured person|first names|middle name|full legal name|\
         legal name of individual|representative|authorised representative|\
         authorized representative|contact name|attn|attention|\
         vollständiger name|kontoinhaber|kontoinhaberin|ansprechpartner|ansprechpartnerin|\
         kontaktperson|antragsteller|antragstellerin|versicherungsnehmer|versicherter|\
         versicherte person|kreditnehmer|darlehensnehmer|mieter|mieterin|kundenname|kunde|\
         sachbearbeiter|bearbeiter|patient|patientin|arbeitnehmer|mitarbeiter|\
         volledige naam|rekeninghouder|contactpersoon|aanvrager|verzekerde|verzekeringnemer|\
         polishouder|policyhouder|kredietnemer|klantnaam|huurder|werknemer|patiënt|\
         nombre y apellidos|apellidos y nombre|titular de la cuenta|nombre del titular|\
         persona de contacto|solicitante|asegurado|tomador|prestatario|arrendatario|\
         empleado|paciente|nombre del cliente|nome e cognome|cognome e nome|intestatario|\
         titolare del conto|referente|richiedente|assicurato|contraente|mutuatario|\
         conduttore|dipendente|paziente|nome del cliente|namn|fullständigt namn|förnamn|\
         efternamn|kontoinnehavare|kontaktperson|sökande|försäkringstagare|försäkrad|\
         låntagare|kundnamn|hyresgäst|policyhållare|försäkringsinnehavare|anställd|\
         handläggare|nom et prénom|prénom et nom|titulaire du compte|nom du titulaire|\
         personne de contact|interlocuteur|demandeur|assuré|nom de l'assuré|\
         prénom de l'assuré|souscripteur|emprunteur|locataire|salarié|employé|\
         nom du client|médecin traitant|kund|policyholder|policy holder|insured name",
        FieldKind::Person,
    ),
    (
        "account name|party|party name|payee|payer|beneficiary|remitter|ordering customer|debtor|creditor|sender|receiver|\
         holder|account owner|owner|landlord|lessor|lessee|guarantor|signatory|signed|\
         signed by|signature|authorised by|authorized by|approved by|prepared by|\
         submitted by|requested by|reported by|reviewed by|issued to|assigned to|\
         counterparty|investor|shareholder|insured|borrower|lender|consignee|shipper|\
         notify party|buyer|seller|supplier|vendor|merchant|advisor|adviser|broker|\
         empfänger|absender|auftraggeber|zahler|zahlungsempfänger|begünstigter|inhaber|\
         eigentümer|vermieter|unterschrift|unterzeichnet|käufer|verkäufer|lieferant|\
         ontvanger|begunstigde|afzender|opdrachtgever|betaler|eigenaar|verhuurder|\
         handtekening|ondertekend door|ondertekend|getekend|koper|verkoper|leverancier|tegenpartij|\
         titular|beneficiario|destinatario|remitente|ordenante|pagador|propietario|\
         arrendador|firmado|firmado por|comprador|vendedor|proveedor|contraparte|\
         mittente|ordinante|pagatore|proprietario|locatore|firmato|firmato da|acquirente|\
         venditore|fornitore|controparte|mottagare|avsändare|betalare|betalningsmottagare|\
         förmånstagare|innehavare|ägare|fastighetsägare|hyresvärd|underskrift|undertecknad|\
         undertecknat|signerat|unterschrieben|signé|firmado|
         köpare|säljare|leverantör|motpart|titulaire|bénéficiaire|expéditeur|donneur d'ordre|\
         payeur|propriétaire|bailleur|signataire|signé par|acheteur|vendeur|fournisseur|\
         contrepartie|firma",
        FieldKind::Party,
    ),
    (
        "bank|bank name|name of bank|banking institution|financial institution|institution|\
         issuer|issuer name|issuing bank|insurer|insurance company|carrier|employer name|\
         business name|business|legal entity|entity|entity name|company legal name|\
         reporting entity|fund|fund name|manufacturer|organisation name|organization name|\
         firm|firm name|corporation|institution name|custodian|registrar|trustee|\
         name der bank|kreditinstitut|versicherer|versicherungsgesellschaft|firmenname|\
         hersteller|arbeitgebername|naam van de bank|bank naam|verzekeraar|bedrijfsnaam|\
         fabrikant|nombre del banco|banco|entidad|entidad bancaria|aseguradora|\
         nombre de la empresa|razón social|fabricante|nome della banca|banca|istituto|\
         compagnia|compagnia assicurativa|ragione sociale|denominazione|produttore|\
         bankens namn|bankens|bolag|företag|företagsnamn|försäkringsbolag|arbetsgivare|\
         tillverkare|nom de la banque|banque|établissement|assureur|raison sociale|\
         nom de l'entreprise|fabricant|nome dell'azienda|nome della società|\
         nome dell'impresa|nombre de la compañía|nom de la société|naam van het bedrijf|\
         name der firma|företagets namn|compagnie de transport|compagnie maritime|\
         transporteur|fraktbolag|fraktör|spediteur|frachtführer|transportista|vettore|\
         vervoerder",
        FieldKind::Org,
    ),
    (
        "adress|gatuadress|postadress|hemadress|besöksadress|leveransadress|\
         faktureringsadress|straatadres|postadres|woonadres|bezoekadres|afleveradres|\
         factuuradres|straße|strasse|straße und hausnummer|wohnanschrift|postanschrift|\
         lieferadresse|rechnungsadresse|objektadresse|geschäftsadresse|firmenadresse|\
         dirección postal|direccion postal|dirección de envío|dirección de facturación|\
         calle|indirizzo di residenza|indirizzo di spedizione|indirizzo di fatturazione|\
         sede legale|sede|adresse postale|adresse de livraison|adresse de facturation|\
         siège social|siège|domicile|street|address line 1|address line|address1|addr|\
         registered office|registered address|head office|business address|\
         company address|employer address|property address|permanent address|\
         current address|correspondence address|residence|residential|location address",
        FieldKind::Address,
    ),
    (
        "user|login name|screen name|handle|\
         benutzername|benutzer|nutzername|anmeldename|gebruikersnaam|gebruiker|\
         nombre de usuario|usuario|nome utente|utente|nom d'utilisateur|identifiant|\
         användarnamn|användare|användar id",
        FieldKind::Username,
    ),
    (
        "driver's license|driver's license number|drivers license|drivers license number|\
         driver license|driver license number|driving licence|driving licence number|\
         driving license|license number|licence number|dl|dl number|dl no|\
         passport no|passport nr|social security number|social security no|ssn number|\
         national insurance number|tax id|tax id number|taxpayer id|\
         taxpayer identification number|tin|national id number|id number|\
         identification number|id card number|personal id|personnummer|\
         führerschein|führerscheinnummer|fahrzeugführerscheinnummer|reisepass|\
         reisepassnummer|passnummer|passportnummer|personalausweis|personalausweisnummer|ausweisnummer|\
         steuer id|steueridentifikationsnummer|sozialversicherungsnummer|rijbewijs|\
         rijbewijsnummer|paspoort|paspoortnummer|bsn|burgerservicenummer|\
         identiteitskaart|licencia de conducir|licencia de conducción|licencia conduccion|\
         carnet de conducir|permiso de conducir|pasaporte|número de pasaporte|\
         numero de pasaporte|dni|nie|nif|número de seguridad social|patente di guida|\
         numero di patente|numero di patente di guida|passaporto|\
         numero di passaporto|codice fiscale|carta d'identità|körkort|körkortsnummer|\
         id nummer|permis de conduire|numéro de permis|passeport|\
         numéro de passeport|numéro de sécurité sociale|carte d'identité",
        FieldKind::NationalId,
    ),
];

static LABELS: LazyLock<HashMap<String, FieldKind>> = LazyLock::new(|| {
    let mut m: HashMap<String, FieldKind> = HashMap::new();
    for (labels, kind) in MORE_LABELS {
        for l in labels.split('|') {
            m.insert(normalise_label(l), *kind);
        }
    }
    // The original table wins over the broader lists.
    m.extend(FIELD_LABELS.iter().map(|(l, k)| (l.to_string(), *k)));
    m
});

/// Lowercase label words: camelCase split ("DriverLicenseNumber"), `_`/`-` as spaces,
/// Markdown escapes and emphasis dropped ("user\_name", "**Name**").
fn normalise_label(l: &str) -> String {
    let mut out = String::with_capacity(l.len() + 4);
    let mut prev_lower = false;
    for c in l.trim().trim_end_matches('.').chars() {
        if matches!(c, '\\' | '*' | '"' | '`') {
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase();
        out.extend(c.to_lowercase());
    }
    out.replace(['_', '-'], " ")
        .replace('’', "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True for labels that ask for a family name alone ("Last name", "Nachname", "Cognome").
fn surname_label(l: &str) -> bool {
    const SURNAME: &[&str] = &[
        "last name",
        "lastname",
        "surname",
        "family name",
        "nachname",
        "familienname",
        "achternaam",
        "apellido",
        "apellidos",
        "cognome",
        "efternamn",
        "nom de famille",
        "sobrenome",
    ];
    let n = normalise_label(l);
    SURNAME.iter().any(|s| n.ends_with(s))
}

fn label_kind(l: &str) -> Option<FieldKind> {
    let n = normalise_label(l);
    if let Some(k) = LABELS.get(n.as_str()) {
        return Some(*k);
    }
    // Compound labels: "Investor Street Address", "Beneficiary Name", "Card Holder Name".
    let (head, last) = n.rsplit_once(' ')?;
    let head_kind = || LABELS.get(head).copied();
    match last {
        "address" | "adres" | "adresse" | "adress" | "indirizzo" | "dirección" | "direccion"
        | "anschrift" => {
            const NOT_POSTAL: &[&str] = &[
                "email",
                "e mail",
                "mail",
                "ip",
                "mac",
                "web",
                "website",
                "wallet",
                "url",
                "internet",
                "billing email",
                "contact email",
                "server",
                "host",
                "network",
            ];
            (!NOT_POSTAL.iter().any(|w| head.ends_with(w))).then_some(FieldKind::Address)
        }
        "name" | "naam" | "namn" | "nombre" | "nome" | "nom" => match head_kind() {
            Some(FieldKind::Username) => Some(FieldKind::Username),
            Some(FieldKind::Org) => Some(FieldKind::Org),
            Some(_) => Some(FieldKind::Person),
            // "Partner Name", "Product Name": only a value that is clearly a person.
            None => Some(FieldKind::NameOnly),
        },
        "number" | "no" | "nr" | "nummer" | "número" | "numero" | "numéro" => {
            head_kind().filter(|k| *k == FieldKind::NationalId)
        }
        _ => None,
    }
}

/// `Label: value` at the start of a line (optionally bulleted).
static FIELD_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^[^\S\n]*(?:[-*•][^\S\n]+|\d{1,2}(?:\.\d{1,2})*\.?[^\S\n]+)?(?:\*\*|__)?[\[{]?([\p{L}][\p{L} _.\-'’\\]{0,40}?)[\]}]?[^\S\n]*(?:\*\*|__)?[:：](?:\*\*|__)?[^\S\n]*([^\s][^\n]*?)[^\S\n]*$",
    )
    .expect("valid regex")
});

/// `"key": "value"` pairs (JSON).
static JSON_PAIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""([A-Za-z][A-Za-z0-9 _\-\\]{0,40})"[^\S\n]*:[^\S\n]*"([^"\n]{1,160})""#)
        .expect("valid regex")
});

/// `<Tag attr="...">value</Tag>` elements (XML), tag names checked to match in code.
static XML_ELEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"<([A-Za-z][\w:.\-]{0,40})(?:[^\S\n][^<>\n]*)?>([^<>\n]{1,160})</([A-Za-z][\w:.\-]{0,40})>",
    )
    .expect("valid regex")
});

/// `Label: value` inside a delimited line ("User ID: x, Action: y, Account: Jane Roe").
static INLINE_PAIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^\S\n]*([\p{L}][\p{L} _.\-'’]{0,30}?)[^\S\n]*:[^\S\n]*([^\s][^\n]*?)[^\S\n]*$")
        .expect("valid regex")
});

/// The start of a street line: a house number and a capitalised word, or a street type.
static STREET_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[^\S\n]*(?:\d{1,5}[A-Za-z]?(?:/\d{1,4})?,?[^\S\n]+\p{Lu}|(?:Calle|Via|Viale|Rue|Avenida|Piazza|Corso|Paseo|Plaza|Rua|Strada|Avenue|Carrer|Rambla)[^\S\n]+\p{Lu})",
    )
    .expect("valid regex")
});

/// A letter's closing line ("Best regards,", "Mit freundlichen Grüßen", "Cordiali saluti").
static CLOSING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)^[^\S\n]*(?:best regards|kind regards|warm regards|regards|best wishes|sincerely|yours sincerely|sincerely yours|yours faithfully|yours truly|respectfully|cheers|many thanks|thanks|thank you|mit freundlichen gr(?:ü|ue)(?:ß|ss)en|freundliche gr(?:ü|ue)(?:ß|ss)e|viele gr(?:ü|ue)(?:ß|ss)e|beste gr(?:ü|ue)(?:ß|ss)e|mfg|met vriendelijke groet(?:en)?|vriendelijke groet(?:en)?|hartelijke groet(?:en)?|groeten|cordialement|bien cordialement|sincères salutations|meilleures salutations|bien à vous|atentamente|saludos(?: cordiales)?|un saludo|cordiali saluti|distinti saluti|saluti|cordialmente|med vänliga hälsningar|vänliga hälsningar|med vänlig hälsning|hälsningar|mvh)[^\S\n]*[,.!]?[^\S\n]*$",
    )
    .expect("valid regex")
});

const PLACEHOLDER_VALUES: &[&str] = &[
    "n/a", "na", "none", "null", "unknown", "tbd", "todo", "-", "redacted", "string", "str",
    "true", "false", "required", "optional",
];

/// Validate a field value for its kind; returns the kind to emit.
fn field_value_kind(fk: FieldKind, v: &str, strong: bool, surname: bool) -> Option<Kind> {
    let lower = v.to_lowercase();
    // Template slots ("[Bank Name]", "{name}", "<Employer>", "____") hold nothing yet, and a
    // value that is itself a field label ("Indirizzo", "Bank Name") is a heading.
    if v.is_empty()
        || label_kind(v).is_some()
        || PLACEHOLDER_VALUES.contains(&lower.as_str())
        || v.contains("{{")
        || v.contains("__")
        || v.starts_with(['[', '{', '<'])
    {
        return None;
    }
    let digits = v.chars().filter(char::is_ascii_digit).count();
    // A name field that holds a company ("Account Name: Globex Ltd") is an organisation.
    let org_like = || {
        v.chars().count() <= 80
            && v.starts_with(|c: char| c.is_uppercase() || c.is_ascii_digit())
            && !v.contains('@')
            && (is_org_name(v)
                || (v.split_whitespace().any(is_org_word) && v.split_whitespace().any(distinctive)))
    };
    // "Namn: gaetano97", "Payee: ryan_488": a person field holding an account handle.
    let handle = !v.contains(char::is_whitespace)
        && (4..=32).contains(&v.len())
        && v.starts_with(|c: char| c.is_ascii_lowercase())
        && v.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
        && v.chars().any(|c| c.is_ascii_digit() || c == '_');
    match fk {
        FieldKind::Person | FieldKind::Name | FieldKind::Party | FieldKind::NameOnly if handle => {
            Some(Kind::Username)
        }
        FieldKind::Person | FieldKind::Name | FieldKind::Party if org_like() => Some(Kind::Org),
        FieldKind::NameOnly if org_like() => None,
        FieldKind::Party | FieldKind::NameOnly => {
            let ok = !v.contains('@')
                && digits == 0
                && v.chars().count() <= 60
                && (2..=5).contains(&v.split_whitespace().count())
                && looks_like_name(v)
                && (v
                    .split_whitespace()
                    .any(|t| is_first_name(t) || is_initial(t))
                    || v.split_whitespace().any(|t| PARTICLES.contains(&t))
                    || v.split_whitespace().last().is_some_and(double_barrelled));
            ok.then_some(Kind::Person)
        }
        FieldKind::Person | FieldKind::Name => {
            let tokens = v.split_whitespace().count();
            // One word ("Customer: Sure", "Name: Titane") must be a known given name, unless
            // the label asks for a surname.
            if tokens == 1 && !surname && !is_first_name(v) && !v.chars().any(is_cjk) {
                return None;
            }
            let ok = !v.contains('@')
                && digits == 0
                && v.chars().count() <= 60
                && tokens <= 6
                && looks_like_name(v)
                && (fk == FieldKind::Person
                    || strong
                    || v.split_whitespace()
                        .any(|t| is_first_name(t) || is_initial(t))
                    || v.chars().any(is_cjk));
            ok.then_some(Kind::Person)
        }
        FieldKind::Address => {
            let ok = v.chars().count() <= 160
                && (digits > 0 || v.chars().any(is_cjk))
                && v.chars().filter(|c| c.is_alphabetic()).count() >= 3
                && !v.contains('@')
                && !v.contains("://");
            ok.then_some(Kind::Address)
        }
        FieldKind::Phone => phone_shape(v).then_some(Kind::Phone),
        FieldKind::Date => parse_date(v).map(|_| Kind::Date),
        FieldKind::Org => {
            let ok = v.chars().count() <= 80
                && v.chars().any(|c| c.is_uppercase() || is_cjk(c))
                && !v.contains('@')
                && !v.contains("://");
            // "Issuer Name: Eva S. Fallaci": an individual in an organisation field.
            let person = !org_like()
                && (2..=4).contains(&v.split_whitespace().count())
                && looks_like_name(v)
                && v.split_whitespace()
                    .any(|t| is_first_name(t) || is_initial(t));
            ok.then_some(if person { Kind::Person } else { Kind::Org })
        }
        FieldKind::Username => {
            let ok = !v.contains(char::is_whitespace)
                && (2..=64).contains(&v.len())
                && v.chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-'))
                && !matches!(
                    lower.as_str(),
                    "admin" | "root" | "user" | "username" | "guest"
                );
            ok.then_some(Kind::Username)
        }
        FieldKind::NationalId => {
            let ok = digits >= 4 && v.len() <= 24 && !dummy_number(v);
            ok.then_some(Kind::NationalId)
        }
    }
}

pub(crate) fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF)
}

/// Labelled fields, CSV columns and signature names.
pub struct FieldDetector {
    toggles: DetectorToggles,
}

impl FieldDetector {
    /// Emit only kinds whose own detector toggle is on.
    pub fn new(toggles: &DetectorToggles) -> Self {
        Self {
            toggles: toggles.clone(),
        }
    }

    fn enabled(&self, k: Kind) -> bool {
        let t = &self.toggles;
        match k {
            Kind::Person => t.person,
            Kind::Address => t.address,
            Kind::Phone => t.phone,
            Kind::Date => t.date_of_birth,
            Kind::Org => t.org,
            Kind::Username => t.username,
            Kind::NationalId => t.national_id,
            _ => true,
        }
    }

    fn lines(text: &str, out: &mut Vec<Finding>) {
        for c in FIELD_LINE.captures_iter(text) {
            let (label, value) = (c.get(1).unwrap(), c.get(2).unwrap());
            let Some(fk) = label_kind(label.as_str()) else {
                continue;
            };
            let v = value.as_str().trim_end_matches([',', ';']);
            // A capitalised form label ("Name:") is a form; "name:" may be YAML/code.
            let form = label.as_str().starts_with(char::is_uppercase);
            let surname = surname_label(label.as_str());
            if let Some(kind) = field_value_kind(fk, v, form, surname) {
                out.push(Finding {
                    score: 0.85,
                    ..Finding::new(value.start(), value.start() + v.len(), kind, "fields")
                });
            }
        }
    }

    /// Emit the finding for a labelled value at `start`, if the value fits the label.
    fn emit(out: &mut Vec<Finding>, label: &str, start: usize, v: &str, strong: bool) {
        let Some(fk) = label_kind(label) else {
            return;
        };
        let t = v.trim_end_matches([',', ';', ' ']);
        let lead = t.len() - t.trim_start().len();
        let t = t.trim_start();
        if let Some(kind) = field_value_kind(fk, t, strong, surname_label(label)) {
            out.push(Finding {
                score: 0.85,
                ..Finding::new(start + lead, start + lead + t.len(), kind, "fields")
            });
        }
    }

    /// JSON `"key": "value"` pairs and XML `<Key>value</Key>` elements whose key names a
    /// field ("first_name", "driverLicenseNumber", `<IssuerName>`).
    fn structured(text: &str, out: &mut Vec<Finding>) {
        if text.contains("\":") || text.contains("\" :") {
            for c in JSON_PAIR.captures_iter(text) {
                let (k, v) = (c.get(1).unwrap(), c.get(2).unwrap());
                Self::emit(out, k.as_str(), v.start(), v.as_str(), false);
            }
        }
        if text.contains("</") {
            for c in XML_ELEM.captures_iter(text) {
                let (open, v, close) = (c.get(1).unwrap(), c.get(2).unwrap(), c.get(3).unwrap());
                if open.as_str() != close.as_str() {
                    continue;
                }
                let tag = open.as_str().rsplit(':').next().unwrap_or("");
                Self::emit(out, tag, v.start(), v.as_str(), false);
            }
        }
    }

    /// `Label: value` pairs separated by `,`, `;` or `|` on one line (logs, flat records).
    fn inline_pairs(text: &str, out: &mut Vec<Finding>) {
        let mut offset = 0;
        for line in text.split('\n') {
            let base = offset;
            offset += line.len() + 1;
            if line.matches(':').count() < 2 || !line.contains([',', ';', '|']) {
                continue;
            }
            let mut seg_start = 0;
            for (i, ch) in line.char_indices().chain([(line.len(), ',')]) {
                if !matches!(ch, ',' | ';' | '|') {
                    continue;
                }
                let seg = &line[seg_start..i];
                let at = base + seg_start;
                seg_start = i + ch.len_utf8();
                let Some(c) = INLINE_PAIR.captures(seg) else {
                    continue;
                };
                let (label, value) = (c.get(1).unwrap(), c.get(2).unwrap());
                let form = label.as_str().starts_with(char::is_uppercase);
                Self::emit(
                    out,
                    label.as_str(),
                    at + value.start(),
                    value.as_str(),
                    form,
                );
            }
        }
    }

    /// CSV/TSV blocks: a header row naming the columns, then rows with the same field count.
    fn tables(text: &str, out: &mut Vec<Finding>) {
        let mut offset = 0;
        let lines: Vec<(usize, &str)> = text
            .split('\n')
            .map(|l| {
                let o = offset;
                offset += l.len() + 1;
                (o, l)
            })
            .collect();
        let mut i = 0;
        while i < lines.len() {
            let (_, header) = lines[i];
            let Some(delim) = [',', ';', '\t', '|']
                .into_iter()
                .find(|d| header.matches(*d).count() >= 1)
            else {
                i += 1;
                continue;
            };
            let cols: Vec<&str> = header.split(delim).map(str::trim).collect();
            let is_header = cols.len() >= 2
                && cols.iter().all(|c| {
                    !c.is_empty()
                        && c.len() <= 32
                        && c.chars()
                            .all(|ch| ch.is_alphabetic() || " _-.\\\"'".contains(ch))
                });
            let kinds: Vec<Option<FieldKind>> = cols.iter().map(|c| label_kind(c)).collect();
            let surnames: Vec<bool> = cols.iter().map(|c| surname_label(c)).collect();
            // A two-column "key,value" list ("Ragione Sociale,Nome azienda") is not a table
            // header unless both columns are known labels.
            let labelled = kinds.iter().filter(|k| k.is_some()).count();
            if !is_header || labelled == 0 || (cols.len() == 2 && labelled < 2) {
                i += 1;
                continue;
            }
            i += 1;
            while i < lines.len() {
                let (start, row) = lines[i];
                let row = row.trim_end_matches('\r');
                let fields = split_row(row, delim);
                if row.trim().is_empty() || fields.len() != cols.len() {
                    break;
                }
                for (((a, b), fk), surname) in fields.iter().zip(&kinds).zip(&surnames) {
                    let Some(fk) = fk else { continue };
                    let v = &row[*a..*b];
                    if let Some(kind) = field_value_kind(*fk, v, true, *surname) {
                        out.push(Finding {
                            score: 0.85,
                            ..Finding::new(start + a, start + b, kind, "fields")
                        });
                    }
                }
                i += 1;
            }
        }
    }

    /// The sender's name on the first non-empty line after a closing ("Best regards,").
    fn closings(text: &str, out: &mut Vec<Finding>) {
        for m in CLOSING.find_iter(text) {
            let mut pos = m.end();
            // The name follows within the next three lines (blank lines skipped).
            for _ in 0..3 {
                let Some(nl) = text[pos..].find('\n') else {
                    break;
                };
                pos += nl + 1;
                let line = text[pos..].split('\n').next().unwrap_or("");
                let v = line.trim().trim_matches('*').trim_end_matches(',').trim();
                if v.is_empty() {
                    continue;
                }
                let tokens = v.split_whitespace().count();
                if (2..=4).contains(&tokens)
                    && v.chars().count() <= 60
                    && !v.chars().any(|c| c.is_ascii_digit() || c == '@')
                    && looks_like_name(v)
                    && !v.split_whitespace().any(is_org_word)
                    && !is_org_name(v)
                {
                    let start = pos + line.find(v).unwrap_or(0);
                    out.push(Finding {
                        score: 0.8,
                        ..Finding::new(start, start + v.len(), Kind::Person, "fields")
                    });
                }
                break;
            }
        }
    }

    /// Address blocks: a line holding only a name, right above a street line
    /// ("John Doe\n123 Maple Street\nAnytown").
    fn address_blocks(text: &str, out: &mut Vec<Finding>) {
        let mut offset = 0;
        let mut prev: Option<(usize, &str)> = None;
        for line in text.split('\n') {
            let start = offset;
            offset += line.len() + 1;
            if let Some((pstart, pline)) = prev {
                if STREET_LINE.is_match(line) {
                    let v = pline.trim().trim_matches('*').trim_end_matches(',').trim();
                    let tokens = v.split_whitespace().count();
                    if (2..=4).contains(&tokens)
                        && v.chars().count() <= 50
                        && !v
                            .chars()
                            .any(|c| c.is_ascii_digit() || c == '@' || c == ':')
                        && looks_like_name(v)
                        && !v.split_whitespace().any(is_org_word)
                        && !is_org_name(v)
                    {
                        let s = pstart + pline.find(v).unwrap_or(0);
                        out.push(Finding {
                            score: 0.75,
                            ..Finding::new(s, s + v.len(), Kind::Person, "fields")
                        });
                    }
                }
            }
            prev = Some((start, line));
        }
    }

    /// The first line after a `--` signature separator is the sender's name.
    fn signatures(text: &str, out: &mut Vec<Finding>) {
        let mut pos = 0;
        while let Some(i) = text[pos..].find("\n--") {
            let sep = pos + i + 1;
            pos = sep + 2;
            let rest = &text[sep + 2..];
            let Some(nl) = rest.find('\n') else { break };
            if !rest[..nl].trim().is_empty() {
                continue;
            }
            let line_start = sep + 2 + nl + 1;
            let line = text[line_start..].split('\n').next().unwrap_or("");
            let v = line.trim_end();
            let lead = v.len() - v.trim_start().len();
            let v = v.trim_start();
            if !v.is_empty()
                && v.chars().count() <= 60
                && !v.chars().any(|c| c.is_ascii_digit() || c == '@')
                && looks_like_name(v)
            {
                out.push(Finding {
                    score: 0.8,
                    ..Finding::new(
                        line_start + lead,
                        line_start + lead + v.len(),
                        Kind::Person,
                        "fields",
                    )
                });
            }
        }
    }
}

/// Byte ranges of the trimmed fields of a delimited row (double quotes respected).
fn split_row(row: &str, delim: char) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let (mut start, mut quoted) = (0, false);
    for (i, c) in row.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if c == delim && !quoted {
            out.push((start, i));
            start = i + c.len_utf8();
        }
    }
    out.push((start, row.len()));
    out.into_iter()
        .map(|(a, b)| {
            let f = &row[a..b];
            let lead = f.len() - f.trim_start().len();
            let t = f.trim();
            let (a, t) = (a + lead, t);
            if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
                (a + 1, a + t.len() - 1)
            } else {
                (a, a + t.len())
            }
        })
        .collect()
}

impl Detector for FieldDetector {
    fn name(&self) -> &'static str {
        "fields"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let mut out = vec![];
        if text.contains(':') || text.contains('：') {
            Self::lines(text, &mut out);
            Self::inline_pairs(text, &mut out);
        }
        if text.contains('<') || text.contains('"') {
            Self::structured(text, &mut out);
        }
        if text.contains('\n') {
            Self::tables(text, &mut out);
            if text.contains("\n--") {
                Self::signatures(text, &mut out);
            }
            Self::closings(text, &mut out);
            Self::address_blocks(text, &mut out);
        }
        out.retain(|f| self.enabled(f.kind));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(d: &dyn Detector, t: &str) -> Vec<(String, Kind)> {
        d.detect(t)
            .iter()
            .map(|f| (t[f.start..f.end].to_string(), f.kind))
            .collect()
    }

    #[test]
    fn dates_of_birth_need_context() {
        let d = BirthDateDetector;
        let v = |t: &str| found(&d, t).into_iter().map(|x| x.0).collect::<Vec<_>>();
        assert_eq!(v("Date of birth: 1987-03-04\n"), vec!["1987-03-04"]);
        assert_eq!(v("DOB 04/03/1987, SSN"), vec!["04/03/1987"]);
        assert_eq!(v("She was born on March 4, 1987."), vec!["March 4, 1987"]);
        assert_eq!(v("born 4 March 1987"), vec!["4 March 1987"]);
        assert_eq!(v("Geburtsdatum: 04.03.1987"), vec!["04.03.1987"]);
        assert_eq!(v("née le 4 mars 1987 à Lyon"), vec!["4 mars 1987"]);
        assert_eq!(
            v("fecha de nacimiento: 4 de marzo de 1987"),
            vec!["4 de marzo de 1987"]
        );
        assert_eq!(v("data di nascita 12/31/1990"), vec!["12/31/1990"]);
        assert!(v("Released on 2024-01-15 with the fix").is_empty());
        assert!(v("born in Paris, moved 2001-02-03").is_empty());
        assert!(v("DOB: 2024-13-45").is_empty());
    }

    #[test]
    fn parses_dates() {
        assert_eq!(parse_date("1987-03-04"), Some((1987, 3, 4)));
        assert_eq!(parse_date("31/12/1990"), Some((1990, 12, 31)));
        assert_eq!(parse_date("12/31/1990"), Some((1990, 12, 31)));
        assert_eq!(parse_date("March 4, 1987"), Some((1987, 3, 4)));
        assert_eq!(parse_date("4. März 1987"), Some((1987, 3, 4)));
        assert_eq!(parse_date("hello"), None);
    }

    #[test]
    fn labelled_fields() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let t = "Check this record:\nName: Whitney Brown\nHome address: 12 Rue de la Paix, 75002 Paris\nMobile: +44 20 7946 0958\nCompany: Initech\nname: build\nStatus: active\n";
        let f = found(&d, t);
        assert!(f.contains(&("Whitney Brown".into(), Kind::Person)));
        assert!(f.contains(&("12 Rue de la Paix, 75002 Paris".into(), Kind::Address)));
        assert!(f.contains(&("+44 20 7946 0958".into(), Kind::Phone)));
        assert!(f.contains(&("Initech".into(), Kind::Org)));
        assert!(!f.iter().any(|x| x.0 == "build" || x.0 == "active"));
        // A generic "name" needs a known first name; a YAML step name is not one.
        assert!(found(&d, "steps:\n  - name: Build Docker Image\n").is_empty());
    }

    #[test]
    fn csv_columns() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let t = "Report:\nname,email,phone,city,plan\nLiana Martucci,l@x.it,351338697,Prade,pro\nKaran Raval,k@y.in,+91 98765 43210,Pune,free\n\nTotal: 2";
        let f = found(&d, t);
        assert!(f.contains(&("Liana Martucci".into(), Kind::Person)));
        assert!(f.contains(&("351338697".into(), Kind::Phone)));
        assert!(f.contains(&("+91 98765 43210".into(), Kind::Phone)));
        assert!(!f.iter().any(|x| x.0 == "Prade" || x.0 == "pro"));
        // Columns without a known label are ignored.
        assert!(found(&d, "id,plan\n12,pro\n").is_empty());
    }

    #[test]
    fn labels_across_languages_and_formats() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let f = |t: &str| found(&d, t);
        let t = "**Adress:** 47 Kyrastraat, 2105IL, Vilsteren\nKontoinhaber: Ingrid Pohlmann\nEfternamn: Johansson\nNamn: gaetano97\n3.1 Führerscheinnummer: 59-904732-29\n";
        let got = f(t);
        assert!(got.contains(&("47 Kyrastraat, 2105IL, Vilsteren".into(), Kind::Address)));
        assert!(got.contains(&("Ingrid Pohlmann".into(), Kind::Person)));
        assert!(got.contains(&("Johansson".into(), Kind::Person)));
        assert!(got.contains(&("gaetano97".into(), Kind::Username)));
        assert!(got.contains(&("59-904732-29".into(), Kind::NationalId)));
        // JSON keys, XML elements and inline pairs.
        assert_eq!(
            f(r#"{"driver_license_number": "G07685482", "last_name": "Doe"}"#),
            vec![
                ("G07685482".to_string(), Kind::NationalId),
                ("Doe".to_string(), Kind::Person)
            ]
        );
        assert_eq!(
            f("<IssuerName>Eva S. Fallaci</IssuerName>"),
            vec![("Eva S. Fallaci".to_string(), Kind::Person)]
        );
        assert!(
            f("System,\"User ID: 7, Action: Login, Customer: Amleto Scarfoglio, IP\"")
                .contains(&("Amleto Scarfoglio".to_string(), Kind::Person))
        );
    }

    #[test]
    fn parties_and_placeholders() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let f = |t: &str| found(&d, t);
        // A party is a person or an organisation, by the value.
        assert_eq!(
            f("Payee: Marta Lindqvist\n"),
            vec![("Marta Lindqvist".to_string(), Kind::Person)]
        );
        assert_eq!(
            f("Payee: Caffe Bellini SRL\n"),
            vec![("Caffe Bellini SRL".to_string(), Kind::Org)]
        );
        assert!(f("Payee: Rental Income\n").is_empty());
        // One word must be a given name unless the label asks for a surname.
        assert!(f("Name: Titane\nCustomer: Sure, here it is\n").is_empty());
        // Template slots, label-like values and key/value lists are not data.
        assert!(f("Bank Name: [Bank Name]\nEmployer: ______\n").is_empty());
        assert!(f("<issuer>Bank Name</issuer>").is_empty());
        assert!(f("Ragione Sociale,Nome azienda\nIndirizzo,Indirizzo\nCassa,Cassa\n").is_empty());
    }

    #[test]
    fn closings_and_address_blocks() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let f = |t: &str| found(&d, t);
        assert_eq!(
            f("Thanks.\n\nBest Regards,\n\nNéstor Teruel\nSenior Manager"),
            vec![("Néstor Teruel".to_string(), Kind::Person)]
        );
        assert_eq!(
            f("Med vänliga hälsningar,\nDolores S. Fiorucci\n"),
            vec![("Dolores S. Fiorucci".to_string(), Kind::Person)]
        );
        assert!(f("Kind regards,\nCustomer Service Team\n").is_empty());
        assert_eq!(
            f("Send it to:\nHerminio Crespo\n16987 Paul Village\nJacksonville"),
            vec![("Herminio Crespo".to_string(), Kind::Person)]
        );
    }

    #[test]
    fn signature_names() {
        let d = FieldDetector::new(&DetectorToggles::default());
        let t = "Fix this:\n\n--\nMaria Tins-van der Flaas\nPsychologist\n";
        assert_eq!(
            found(&d, t),
            vec![("Maria Tins-van der Flaas".to_string(), Kind::Person)]
        );
        assert!(found(&d, "a\n--\nsee the docs above\n").is_empty());
    }
}
