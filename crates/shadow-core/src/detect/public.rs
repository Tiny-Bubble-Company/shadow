//! Public-entity allowlist.
//!
//! Name-like detectors (ML sidecars, rule-based names) tend to flag well-known public names:
//! "Microsoft", "Paris", "Satya Nadella", "ISO 27001". Masking those hides useful context
//! and protects nobody, so [`suppress`] drops such findings when the *whole* span is a
//! curated public entity (case-insensitive). It never touches user-configured dictionary
//! terms or structured kinds (emails, IDs, cards...).

use std::collections::HashSet;
use std::sync::LazyLock;

use super::{Finding, Kind};

/// Well-known companies, products, services and institutions.
const ORGS: &str = "\
Microsoft|Microsoft Corporation|Microsoft Azure|Azure|Google|Google Cloud|Google Cloud Platform|GCP|Alphabet|Amazon|Amazon Web Services|AWS|Apple|Meta|Facebook|Instagram|WhatsApp|Netflix|OpenAI|Anthropic|Claude|ChatGPT|GPT-4|GPT-4o|Gemini|Bard|Copilot|GitHub Copilot|Mistral|Mistral AI|Cohere|Hugging Face|DeepMind|Nvidia|NVIDIA|AMD|Intel|Qualcomm|IBM|Oracle|SAP|Salesforce|Adobe|Cisco|HP|Hewlett Packard Enterprise|HPE|Lenovo|Samsung|Sony|Panasonic|Toshiba|Hitachi|Fujitsu|NEC|Canon|Nikon|Nintendo|Toyota|Honda|Nissan|Mazda|Subaru|Mitsubishi|Suzuki|SoftBank|Rakuten|Uniqlo|Tesla|SpaceX|Twitter|X Corp|LinkedIn|Snapchat|TikTok|ByteDance|Pinterest|Reddit|Discord|Telegram|Zoom|Slack|Microsoft Teams|Teams|Atlassian|Jira|Confluence|Trello|Bitbucket|GitHub|GitLab|Notion|Figma|Canva|Miro|Asana|Monday.com|Airtable|Dropbox|Stripe|PayPal|Visa|Mastercard|American Express|Amex|Adyen|Klarna|Revolut|N26|Coinbase|Binance|Shopify|WordPress|Wix|Squarespace|Zendesk|HubSpot|Mailchimp|Twilio|SendGrid|Okta|Auth0|Cloudflare|Akamai|Fastly|Vercel|Netlify|Heroku|DigitalOcean|Linode|OVH|OVHcloud|Hetzner|Snowflake|Databricks|Palantir|Tableau|Power BI|Looker|Splunk|Datadog|New Relic|Grafana|Grafana Labs|Elasticsearch|Kibana|Logstash|MongoDB|Redis|PostgreSQL|Postgres|MySQL|MariaDB|SQLite|SQL Server|Cassandra|Kafka|Apache Kafka|Apache|Apache Spark|Hadoop|Airflow|dbt|Terraform|HashiCorp|Consul|Ansible|Jenkins|CircleCI|Travis CI|GitHub Actions|Docker|Docker Hub|Kubernetes|K8s|Helm|Istio|OpenShift|Red Hat|Canonical|Ubuntu|Debian|Fedora|CentOS|Linux|Windows|Windows 10|Windows 11|macOS|iOS|iPadOS|Android|ChromeOS|Chrome|Google Chrome|Firefox|Mozilla|Safari|Microsoft Edge|Opera|Brave|Outlook|Gmail|Google Drive|Google Docs|Google Sheets|Google Meet|YouTube|Maps|Google Maps|Excel|Microsoft Excel|Microsoft Word|PowerPoint|OneNote|OneDrive|SharePoint|Office 365|Microsoft 365|Visual Studio|Visual Studio Code|VS Code|IntelliJ|JetBrains|PyCharm|Xcode|Eclipse|Vim|Emacs|Python|Rust|Golang|Java|JavaScript|TypeScript|Node.js|Deno|React|Vue|Angular|Svelte|Next.js|Django|Flask|FastAPI|Rails|Ruby on Rails|Ruby|PHP|Laravel|Spring|Spring Boot|.NET|C#|C++|Kotlin|Swift|Scala|Haskell|Elixir|Erlang|Perl|MATLAB|Julia|Pandas|NumPy|PyTorch|TensorFlow|Keras|scikit-learn|LangChain|LlamaIndex|Llama|Ollama|Whisper|Stable Diffusion|Midjourney|DALL-E|Uber|Lyft|Airbnb|Booking.com|Expedia|Tripadvisor|eBay|Alibaba|Alibaba Cloud|Tencent|Baidu|Huawei|Xiaomi|Lenovo|JD.com|Flipkart|Infosys|Tata|Tata Consultancy Services|TCS|Wipro|HCL|Reliance|Reliance Industries|Mahindra|Bajaj|ICICI|HDFC|Paytm|Zomato|Swiggy|Siemens|Bosch|BASF|Bayer|BMW|Mercedes-Benz|Daimler|Volkswagen|Audi|Porsche|Allianz|Deutsche Bank|Commerzbank|Deutsche Telekom|Telekom|Lufthansa|Adidas|Puma|SAP SE|Zalando|Delivery Hero|Infineon|Continental|Henkel|Merck|Airbus|Thales|Dassault|Dassault Systèmes|Renault|Peugeot|Stellantis|Michelin|L'Oréal|LVMH|Louis Vuitton|Chanel|Hermès|Danone|Carrefour|TotalEnergies|BNP Paribas|Société Générale|Crédit Agricole|AXA|Capgemini|Atos|Ubisoft|Sanofi|Schneider Electric|Vinci|Engie|Kering|Telefónica|Santander|Banco Santander|BBVA|Iberdrola|Inditex|Zara|Repsol|CaixaBank|Mercadona|Enel|Eni|Fiat|Ferrari|Lamborghini|Gucci|Prada|Armani|Pirelli|UniCredit|Intesa Sanpaolo|Generali|Luxottica|Barilla|Philips|Shell|ING|ABN AMRO|Rabobank|Heineken|Unilever|ASML|Booking Holdings|Adyen|Ahold Delhaize|KLM|Air France|Petrobras|Itaú|Bradesco|Banco do Brasil|Nubank|Embraer|Ambev|Mercado Libre|Magazine Luiza|BP|HSBC|Barclays|Barclays Bank|Royal Bank of Scotland|Lloyds Bank|Santander UK|Royal Bank of Canada|RBC|TD Bank|Scotiabank|BMO|CIBC|Swedbank|SEB|Nordea|Handelsbanken|Danske Bank|DNB|ING Bank|KBC|Belfius|US Bank|U.S. Bank|PNC|Capital One|Citibank|Chase|Chase Bank|Wells Fargo Bank|Bank of China|Banca d'Italia|Banco de España|Banque de France|De Nederlandsche Bank|Sveriges Riksbank|Riksbank|Amazon.com|Lloyds|NatWest|Standard Chartered|Vodafone|BT|Tesco|Sainsbury's|Unilever|GSK|AstraZeneca|Rolls-Royce|BAE Systems|Arm Holdings|Deliveroo|Revolut|Monzo|BBC|The Guardian|Reuters|Bloomberg|Financial Times|The Economist|New York Times|Washington Post|Wall Street Journal|CNN|Fox News|Nestlé|Novartis|Roche|UBS|Credit Suisse|Zurich Insurance|ABB|Spotify|Ericsson|Volvo|IKEA|H&M|Nokia|Maersk|Novo Nordisk|Lego|Equinor|Walmart|Costco|Home Depot|Starbucks|McDonald's|Coca-Cola|Pepsi|PepsiCo|Nike|Disney|Walt Disney|Warner Bros|Comcast|AT&T|Verizon|T-Mobile|JPMorgan|JPMorgan Chase|Goldman Sachs|Morgan Stanley|Citigroup|Citi|Bank of America|Wells Fargo|BlackRock|Vanguard|Vanguard Group|Fidelity|Fidelity Investments|Charles Schwab|Schwab|T. Rowe Price|State Street|Northern Trust|Edward Jones|Raymond James|Berkshire Hathaway|Johnson & Johnson|Pfizer|Moderna|Boeing|Lockheed Martin|General Electric|GE|General Motors|GM|Exxon|ExxonMobil|Chevron|Procter & Gamble|P&G|Accenture|Deloitte|PwC|KPMG|EY|Ernst & Young|McKinsey|Boston Consulting Group|BCG|Gartner|Forrester|Wikipedia|Wikimedia|Mozilla Foundation|Linux Foundation|Apache Software Foundation|OpenStreetMap|Stack Overflow|Hacker News|Medium|Substack|Quora|Yahoo|Bing|DuckDuckGo|Yandex|Naver|Kakao|LINE|WeChat|Alipay|Gojek|Shopee|Lazada|Rappi|iFood|Glovo|Just Eat|Uber Eats|DoorDash|Instacart|FedEx|UPS|DHL|USPS|Royal Mail|La Poste|Deutsche Post|PostNL|Correos|Poste Italiane|Correios|Japan Post|India Post|European Union|EU|the EU|United Nations|UN|NATO|WHO|World Health Organization|IMF|World Bank|OECD|WTO|European Commission|European Parliament|European Central Bank|ECB|Federal Reserve|Bank of England|Bundesbank|FBI|CIA|NSA|NASA|ESA|CERN|IRS|HMRC|NHS|SEC|FTC|FDA|CDC|ICO|CNIL|BfDI|Harvard|Harvard University|MIT|Stanford|Stanford University|Oxford|University of Oxford|Cambridge|University of Cambridge|Yale|Princeton|Berkeley|UC Berkeley|Caltech|ETH Zurich|ETH|Imperial College|Sorbonne|TU Munich|IIT|IIT Bombay|IIT Delhi|University of Tokyo|Olympics|FIFA|UEFA|NBA|NFL|Premier League|Formula 1|Red Cross|UNICEF|Greenpeace|Amnesty International";

/// Countries, regions, capitals and major cities.
const PLACES: &str = "\
Afghanistan|Albania|Algeria|Andorra|Angola|Argentina|Armenia|Australia|Austria|Azerbaijan|Bahamas|Bahrain|Bangladesh|Barbados|Belarus|Belgium|Belize|Benin|Bhutan|Bolivia|Bosnia|Bosnia and Herzegovina|Botswana|Brazil|Brasil|Brunei|Bulgaria|Burkina Faso|Burundi|Cambodia|Cameroon|Canada|Chad|Chile|China|Colombia|Congo|Costa Rica|Croatia|Cuba|Cyprus|Czechia|Czech Republic|Denmark|Djibouti|Dominican Republic|Ecuador|Egypt|El Salvador|Estonia|Eswatini|Ethiopia|Fiji|Finland|France|Gabon|Gambia|Georgia|Germany|Deutschland|Ghana|Greece|Guatemala|Guinea|Guyana|Haiti|Honduras|Hong Kong|Hungary|Iceland|India|Indonesia|Iran|Iraq|Ireland|Israel|Italy|Italia|Jamaica|Japan|Jordan|Kazakhstan|Kenya|Korea|South Korea|North Korea|Kosovo|Kuwait|Kyrgyzstan|Laos|Latvia|Lebanon|Lesotho|Liberia|Libya|Liechtenstein|Lithuania|Luxembourg|Madagascar|Malawi|Malaysia|Maldives|Mali|Malta|Mauritania|Mauritius|Mexico|México|Moldova|Monaco|Mongolia|Montenegro|Morocco|Mozambique|Myanmar|Namibia|Nepal|Netherlands|the Netherlands|Nederland|Holland|New Zealand|Nicaragua|Niger|Nigeria|North Macedonia|Norway|Oman|Pakistan|Palestine|Panama|Papua New Guinea|Paraguay|Peru|Philippines|Poland|Portugal|Qatar|Romania|Russia|Rwanda|Saudi Arabia|Senegal|Serbia|Singapore|Slovakia|Slovenia|Somalia|South Africa|Spain|España|Sri Lanka|Sudan|Suriname|Sweden|Switzerland|Schweiz|Suisse|Syria|Taiwan|Tajikistan|Tanzania|Thailand|Togo|Tunisia|Turkey|Türkiye|Turkmenistan|Uganda|Ukraine|United Arab Emirates|UAE|United Kingdom|UK|Great Britain|Britain|England|Scotland|Wales|Northern Ireland|United States|United States of America|USA|US|America|Uruguay|Uzbekistan|Venezuela|Vietnam|Yemen|Zambia|Zimbabwe|Europe|Asia|Africa|North America|South America|Latin America|Oceania|Antarctica|Middle East|Scandinavia|Balkans|Caribbean|Silicon Valley|Bay Area|\
Alabama|Alaska|Arizona|Arkansas|California|Colorado|Connecticut|Delaware|Florida|Hawaii|Idaho|Illinois|Indiana|Iowa|Kansas|Kentucky|Louisiana|Maine|Maryland|Massachusetts|Michigan|Minnesota|Mississippi|Missouri|Montana|Nebraska|Nevada|New Hampshire|New Jersey|New Mexico|New York|New York City|NYC|North Carolina|North Dakota|Ohio|Oklahoma|Oregon|Pennsylvania|Rhode Island|South Carolina|South Dakota|Tennessee|Texas|Utah|Vermont|Virginia|Washington|Washington D.C.|Washington DC|West Virginia|Wisconsin|Wyoming|Ontario|Quebec|British Columbia|Alberta|Bavaria|Bayern|Berlin|Hamburg|Hesse|Saxony|Catalonia|Cataluña|Andalusia|Andalucía|Tuscany|Toscana|Lombardy|Lombardia|Sicily|Sardinia|Provence|Normandy|Brittany|Île-de-France|Flanders|Wallonia|Maharashtra|Karnataka|Tamil Nadu|Kerala|Gujarat|Punjab|Rajasthan|Uttar Pradesh|West Bengal|Telangana|São Paulo|Rio de Janeiro|Minas Gerais|Bahia|Hokkaido|Okinawa|Kyushu|\
London|Paris|Berlin|Madrid|Rome|Roma|Lisbon|Lisboa|Amsterdam|Brussels|Bruxelles|Vienna|Wien|Prague|Praha|Warsaw|Budapest|Bucharest|Sofia|Athens|Dublin|Edinburgh|Copenhagen|Stockholm|Oslo|Helsinki|Reykjavik|Tallinn|Riga|Vilnius|Kyiv|Kiev|Moscow|Bern|Zurich|Zürich|Geneva|Genève|Basel|Lausanne|Luxembourg City|Monaco|Valletta|Belgrade|Zagreb|Ljubljana|Bratislava|Sarajevo|Skopje|Tirana|Istanbul|Ankara|Munich|München|Frankfurt|Frankfurt am Main|Cologne|Köln|Stuttgart|Düsseldorf|Dortmund|Essen|Leipzig|Dresden|Hanover|Hannover|Nuremberg|Nürnberg|Bremen|Bonn|Heidelberg|Karlsruhe|Mannheim|Freiburg|Lyon|Marseille|Toulouse|Nice|Nantes|Strasbourg|Montpellier|Bordeaux|Lille|Rennes|Grenoble|Barcelona|Valencia|Seville|Sevilla|Zaragoza|Málaga|Malaga|Bilbao|Granada|Palma|Milan|Milano|Naples|Napoli|Turin|Torino|Florence|Firenze|Venice|Venezia|Bologna|Genoa|Genova|Palermo|Verona|Pisa|Porto|Rotterdam|The Hague|Den Haag|Utrecht|Eindhoven|Groningen|Antwerp|Antwerpen|Ghent|Gent|Manchester|Birmingham|Liverpool|Leeds|Glasgow|Bristol|Sheffield|Newcastle|Nottingham|Leicester|Cardiff|Belfast|Oxford|Cambridge|Brighton|Southampton|Gothenburg|Malmö|Aarhus|Bergen|Kraków|Krakow|Gdańsk|Wrocław|Cluj-Napoca|Thessaloniki|\
Washington|Boston|Chicago|Los Angeles|LA|San Francisco|SF|Seattle|Portland|San Diego|San Jose|Austin|Dallas|Houston|San Antonio|Denver|Phoenix|Las Vegas|Salt Lake City|Atlanta|Miami|Orlando|Tampa|New Orleans|Nashville|Philadelphia|Pittsburgh|Baltimore|Detroit|Cleveland|Minneapolis|St. Louis|Kansas City|Charlotte|Raleigh|Indianapolis|Columbus|Cincinnati|Milwaukee|Honolulu|Anchorage|Sacramento|Oakland|Palo Alto|Mountain View|Menlo Park|Cupertino|Redmond|Brooklyn|Manhattan|Queens|Toronto|Montreal|Montréal|Vancouver|Ottawa|Calgary|Edmonton|Mexico City|Ciudad de México|Guadalajara|Monterrey|Havana|Bogotá|Bogota|Medellín|Lima|Santiago|Buenos Aires|Montevideo|Caracas|Quito|La Paz|Asunción|Brasília|Brasilia|Salvador|Fortaleza|Belo Horizonte|Recife|Curitiba|Porto Alegre|Manaus|Campinas|\
Tokyo|Osaka|Kyoto|Yokohama|Nagoya|Sapporo|Kobe|Fukuoka|Hiroshima|Sendai|Nara|Beijing|Shanghai|Shenzhen|Guangzhou|Hangzhou|Chengdu|Wuhan|Hong Kong|Macau|Taipei|Seoul|Busan|Singapore|Kuala Lumpur|Bangkok|Jakarta|Manila|Hanoi|Ho Chi Minh City|Delhi|New Delhi|Mumbai|Bombay|Bangalore|Bengaluru|Chennai|Madras|Hyderabad|Kolkata|Calcutta|Pune|Ahmedabad|Jaipur|Lucknow|Kochi|Goa|Gurgaon|Gurugram|Noida|Chandigarh|Karachi|Lahore|Islamabad|Dhaka|Colombo|Kathmandu|Dubai|Abu Dhabi|Doha|Riyadh|Jeddah|Tel Aviv|Jerusalem|Tehran|Baghdad|Beirut|Amman|Cairo|Casablanca|Marrakech|Tunis|Algiers|Lagos|Abuja|Accra|Nairobi|Addis Ababa|Kampala|Kigali|Dar es Salaam|Johannesburg|Cape Town|Durban|Pretoria|Sydney|Melbourne|Brisbane|Perth|Adelaide|Canberra|Auckland|Wellington|Christchurch";

/// Widely known public figures.
const PEOPLE: &str = "\
Satya Nadella|Tim Cook|Sundar Pichai|Elon Musk|Jeff Bezos|Andy Jassy|Mark Zuckerberg|Bill Gates|Steve Jobs|Steve Wozniak|Larry Page|Sergey Brin|Sam Altman|Dario Amodei|Daniela Amodei|Demis Hassabis|Jensen Huang|Lisa Su|Pat Gelsinger|Marc Benioff|Larry Ellison|Michael Dell|Arvind Krishna|Reed Hastings|Jack Dorsey|Brian Chesky|Travis Kalanick|Susan Wojcicki|Sheryl Sandberg|Marissa Mayer|Ginni Rometty|Mary Barra|Warren Buffett|Charlie Munger|Jamie Dimon|Larry Fink|Ray Dalio|Peter Thiel|Reid Hoffman|Masayoshi Son|Jack Ma|Pony Ma|Mukesh Ambani|Ratan Tata|Gautam Adani|Narayana Murthy|Nandan Nilekani|Linus Torvalds|Guido van Rossum|Bjarne Stroustrup|James Gosling|Dennis Ritchie|Ken Thompson|Tim Berners-Lee|Vint Cerf|Alan Turing|Ada Lovelace|Grace Hopper|Margaret Hamilton|John von Neumann|Geoffrey Hinton|Yann LeCun|Yoshua Bengio|Andrew Ng|Fei-Fei Li|Ilya Sutskever|Andrej Karpathy|Albert Einstein|Isaac Newton|Marie Curie|Charles Darwin|Nikola Tesla|Stephen Hawking|Galileo Galilei|Leonardo da Vinci|Michelangelo|Pablo Picasso|Vincent van Gogh|Rembrandt|Claude Monet|Wolfgang Amadeus Mozart|Ludwig van Beethoven|Johann Sebastian Bach|William Shakespeare|Charles Dickens|Jane Austen|Mark Twain|Ernest Hemingway|George Orwell|J. K. Rowling|J.K. Rowling|Stephen King|Agatha Christie|Victor Hugo|Gabriel García Márquez|Miguel de Cervantes|Dante Alighieri|Johann Wolfgang von Goethe|Goethe|Franz Kafka|Leo Tolstoy|Haruki Murakami|Paulo Coelho|Joe Biden|Donald Trump|Kamala Harris|Barack Obama|Michelle Obama|Hillary Clinton|Bill Clinton|George W. Bush|Ronald Reagan|John F. Kennedy|Abraham Lincoln|George Washington|Franklin D. Roosevelt|Winston Churchill|Margaret Thatcher|Tony Blair|Boris Johnson|Rishi Sunak|Keir Starmer|Theresa May|King Charles|King Charles III|Queen Elizabeth|Queen Elizabeth II|Prince William|Angela Merkel|Olaf Scholz|Friedrich Merz|Emmanuel Macron|Charles de Gaulle|Pedro Sánchez|Giorgia Meloni|Mario Draghi|Ursula von der Leyen|Christine Lagarde|Mark Rutte|Narendra Modi|Mahatma Gandhi|Jawaharlal Nehru|Xi Jinping|Vladimir Putin|Volodymyr Zelensky|Justin Trudeau|Luiz Inácio Lula da Silva|Jair Bolsonaro|Nelson Mandela|Martin Luther King|Martin Luther King Jr.|Pope Francis|Dalai Lama|Greta Thunberg|Malala Yousafzai|Taylor Swift|Beyoncé|Rihanna|Lady Gaga|Michael Jackson|Elvis Presley|The Beatles|John Lennon|Paul McCartney|Bob Dylan|Lionel Messi|Cristiano Ronaldo|Neymar|Pelé|Diego Maradona|Roger Federer|Rafael Nadal|Novak Djokovic|Serena Williams|LeBron James|Michael Jordan|Usain Bolt|Lewis Hamilton|Max Verstappen|Tom Hanks|Leonardo DiCaprio|Meryl Streep|Oprah Winfrey|Shah Rukh Khan|Amitabh Bachchan|Sachin Tendulkar|Virat Kohli";

/// Standards, regulations and common technical terms.
const TERMS: &str = "\
ISO 27001|ISO/IEC 27001|ISO 27002|ISO 27701|ISO 9001|ISO 14001|ISO 22301|ISO 31000|ISO 42001|ISO 8601|ISO 3166|ISO 4217|ISO 20022|SOC 2|SOC2|SOC 1|SOC 2 Type II|SOC 2 Type 2|GDPR|DSGVO|RGPD|UK GDPR|HIPAA|HITECH|PCI DSS|PCI-DSS|PCI|CCPA|CPRA|LGPD|PIPEDA|PIPL|APPI|DPDP|NIST|NIST CSF|NIST 800-53|FedRAMP|FISMA|CMMC|DORA|NIS2|NIS 2|eIDAS|PSD2|AI Act|EU AI Act|Data Act|DMA|DSA|SOX|Sarbanes-Oxley|GLBA|FERPA|COPPA|CIS|OWASP|OWASP Top 10|CVE|CVSS|MITRE ATT&CK|TISAX|BSI|C5|ENS|HDS|IRAP|Cyber Essentials|OAuth|OAuth 2.0|OpenID|OpenID Connect|OIDC|SAML|SCIM|LDAP|Kerberos|TLS|SSL|mTLS|HTTPS|HTTP|HTTP/2|REST|GraphQL|gRPC|WebSocket|JSON|YAML|XML|CSV|PDF|HTML|CSS|SQL|NoSQL|API|APIs|SDK|CLI|GUI|UI|UX|CI/CD|CI|CD|DevOps|DevSecOps|MLOps|SRE|SLA|SLO|SLI|KPI|OKR|ROI|B2B|B2C|SaaS|PaaS|IaaS|AI|ML|LLM|LLMs|NLP|RAG|GPU|CPU|TPU|RAM|SSD|IoT|VPN|DNS|DHCP|TCP|UDP|IP|IPv4|IPv6|SSH|FTP|SFTP|SMTP|IMAP|POP3|MFA|2FA|SSO|IAM|RBAC|ABAC|KMS|HSM|PKI|JWT|AES|RSA|SHA-256|ECDSA|Unicode|UTF-8|ASCII|Markdown|LaTeX|Git|SVN|Agile|Scrum|Kanban|ITIL|TOGAF|COBIT|PRINCE2|PMP|Six Sigma|OKRs|KPIs|ESG|CSRD|IFRS|GAAP|US GAAP|EBITDA|VAT|GST|IBAN|BIC|SWIFT|SEPA|ACH|EMV|NFC|QR|Q1|Q2|Q3|Q4|FY|YoY|MoM|ETA|EOD|ASAP|FAQ|README|TODO|FIXME";

static PUBLIC: LazyLock<HashSet<String>> = LazyLock::new(|| {
    [ORGS, PLACES, PEOPLE, TERMS]
        .iter()
        .flat_map(|list| list.split('|'))
        .map(normalise)
        .filter(|s| !s.is_empty())
        .collect()
});

/// Lowercase, collapse whitespace, drop a leading "the " and a possessive "'s".
fn normalise(s: &str) -> String {
    let s = s
        .trim()
        .trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '"' | '(' | ')'))
        .to_lowercase();
    let s = s
        .strip_suffix("'s")
        .or_else(|| s.strip_suffix("’s"))
        .unwrap_or(&s)
        .to_string();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.strip_prefix("the ").map(str::to_string).unwrap_or(s)
}

/// True if `value` is (as a whole) a well-known public entity.
pub fn is_public(value: &str) -> bool {
    let n = normalise(value);
    !n.is_empty() && PUBLIC.contains(&n)
}

/// Detectors whose name-like findings the allowlist may drop.
pub(crate) const ML_DETECTORS: &[&str] = &["model", "presidio"];
const NAME_DETECTORS: &[&str] = &["person", "fields", "org"];

/// Whether `finding` (covering `value`) should be dropped as a public entity or ML noise.
/// Only Person/Org/Address/Term findings from ML detectors and the rule-based name
/// detectors are considered; dictionary terms and structured kinds are never touched.
pub fn suppress(finding: &Finding, value: &str) -> bool {
    if !matches!(
        finding.kind,
        Kind::Person | Kind::Org | Kind::Address | Kind::Term
    ) {
        return false;
    }
    let ml = ML_DETECTORS.contains(&finding.detector);
    if !ml && !NAME_DETECTORS.contains(&finding.detector) {
        return false;
    }
    if ml && matches!(finding.kind, Kind::Person | Kind::Org | Kind::Address) {
        // Lowercase header words ("email", "phone") and fragments are not names. Scripts
        // without case (CJK, Arabic, ...) count as capitalised.
        let capitalised = value
            .chars()
            .any(|c| c.is_uppercase() || (c.is_alphabetic() && !c.is_lowercase()));
        if value.trim().chars().count() < 3 || !capitalised {
            return true;
        }
    }
    // "Banco Santander, S.A." is as public as "Banco Santander".
    is_public(value) || is_public(super::dictionary::strip_legal_suffix(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(kind: Kind, detector: &'static str) -> Finding {
        Finding::new(0, 1, kind, detector)
    }

    #[test]
    fn public_names_are_kept() {
        assert!(is_public("Microsoft"));
        assert!(is_public("microsoft"));
        assert!(is_public("Satya Nadella"));
        assert!(is_public("the European Union"));
        assert!(is_public("Google's"));
        assert!(is_public("ISO 27001"));
        assert!(is_public("New  York"));
        assert!(!is_public("Acme Widgets"));
        assert!(!is_public("Jane Doe"));
        // Whole-span only: a public word inside a private name does not count.
        assert!(!is_public("Paris Hilton Consulting"));
    }

    #[test]
    fn suppress_scope() {
        assert!(suppress(&f(Kind::Person, "model"), "Satya Nadella"));
        assert!(suppress(&f(Kind::Org, "presidio"), "Stripe"));
        assert!(suppress(&f(Kind::Address, "model"), "Paris"));
        assert!(suppress(&f(Kind::Person, "person"), "Tim Cook"));
        // A legal form does not make a public company private.
        assert!(suppress(&f(Kind::Org, "org"), "Banco Santander, S.A."));
        // ML noise: lowercase header words and fragments.
        assert!(suppress(&f(Kind::Person, "model"), "email"));
        assert!(suppress(&f(Kind::Org, "presidio"), "Ab"));
        assert!(!suppress(&f(Kind::Person, "model"), "松本 康弘"));
        assert!(!suppress(&f(Kind::Person, "model"), "Jane Doe"));
        // Never: user dictionary terms or structured kinds.
        assert!(!suppress(&f(Kind::Org, "dictionary"), "Microsoft"));
        assert!(!suppress(&f(Kind::Email, "model"), "email"));
        assert!(!suppress(&f(Kind::Hostname, "hostname"), "google"));
    }
}
