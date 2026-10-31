"""Generate Shadow's labelled benchmark set: realistic AI prompts with every sensitive span recorded.

Original and synthetic (no copied data), deterministic (fixed seed), multilingual. Each line:

    {"id", "scenario", "locale", "text",
     "spans":  [{"start", "end", "label", "value"}],   # must be protected
     "public": [{"start", "end", "value"}]}            # public names that should be left alone

Offsets are Python string indices (Unicode code points). Labels:
PERSON EMAIL PHONE ADDRESS ORG IBAN CREDIT_CARD NATIONAL_ID IP HOSTNAME URL SECRET USERNAME DATE_OF_BIRTH

    python generate.py --n 1500 --out synthetic-v1.jsonl
"""

from __future__ import annotations

import argparse
import json
import random
import string

from faker import Faker

LOCALES = ["en_US", "en_GB", "de_DE", "fr_FR", "es_ES", "it_IT", "nl_NL", "pt_BR", "en_IN", "ja_JP"]

PUBLIC = [
    "Microsoft", "Google", "Amazon Web Services", "OpenAI", "Anthropic", "Stripe", "Salesforce", "GitHub", "Slack",
    "Paris", "London", "Berlin", "New York", "Tokyo", "the European Union", "Satya Nadella", "Tim Cook", "Linux",
    "Kubernetes", "PostgreSQL", "ISO 27001", "GDPR", "Excel", "Python", "Rust",
]


class Doc:
    """Builds text while recording the span of every inserted value."""

    def __init__(self) -> None:
        self.parts: list[str] = []
        self.spans: list[dict] = []
        self.public: list[dict] = []
        self.len = 0

    def t(self, s: str) -> "Doc":
        self.parts.append(s)
        self.len += len(s)
        return self

    def e(self, label: str, value: str) -> "Doc":
        self.spans.append({"start": self.len, "end": self.len + len(value), "label": label, "value": value})
        return self.t(value)

    def p(self, value: str) -> "Doc":
        self.public.append({"start": self.len, "end": self.len + len(value), "value": value})
        return self.t(value)

    def text(self) -> str:
        return "".join(self.parts)


class Gen:
    def __init__(self, seed: int) -> None:
        self.r = random.Random(seed)
        self.fakers = {l: Faker(l) for l in LOCALES}
        for i, f in enumerate(self.fakers.values()):
            f.seed_instance(seed + i)

    def f(self, locale: str | None = None) -> Faker:
        return self.fakers[locale or self.r.choice(LOCALES)]

    # ----------------------------------------------------------------- values

    def person(self, fk: Faker) -> str:
        return fk.name().replace("\n", " ")

    def email(self, fk: Faker, name: str | None = None) -> str:
        if name and self.r.random() < 0.6:
            parts = [p for p in name.lower().replace(".", "").split() if p.isascii() and p.isalpha()]
            if len(parts) >= 2:
                return f"{parts[0]}.{parts[-1]}@{self.domain(fk)}"
        return fk.email(safe=False)

    def domain(self, fk: Faker) -> str:
        return fk.domain_name()

    def phone(self, fk: Faker) -> str:
        return fk.phone_number()

    def address(self, fk: Faker) -> str:
        return fk.address().replace("\n", ", ")

    def org(self, fk: Faker) -> str:
        name = fk.company()
        if self.r.random() < 0.5:
            name += self.r.choice([" GmbH", " Ltd", " Inc.", " LLC", " S.A.", " B.V.", " AG", " SAS", " Pvt Ltd"])
        return name

    def iban(self, fk: Faker) -> str:
        raw = fk.iban()
        return " ".join(raw[i : i + 4] for i in range(0, len(raw), 4)) if self.r.random() < 0.5 else raw

    def card(self, fk: Faker) -> str:
        n = fk.credit_card_number(card_type=self.r.choice(["visa", "mastercard", "amex"]))
        return " ".join(n[i : i + 4] for i in range(0, len(n), 4)) if self.r.random() < 0.5 and len(n) == 16 else n

    def national_id(self) -> tuple[str, str]:
        kind = self.r.choice(["US SSN", "UK NINO", "IN Aadhaar", "BR CPF", "NL BSN", "ES NIF", "IT CF", "FR INSEE"])
        f = {
            "US SSN": lambda: self.fakers["en_US"].ssn(),
            "UK NINO": lambda: self.fakers["en_GB"].ssn(),
            "IN Aadhaar": lambda: self.fakers["en_IN"].aadhaar_id(),
            "BR CPF": lambda: self.fakers["pt_BR"].cpf(),
            "NL BSN": lambda: self.fakers["nl_NL"].ssn(),
            "ES NIF": lambda: self.fakers["es_ES"].nif(),
            "IT CF": lambda: self.fakers["it_IT"].ssn(),
            "FR INSEE": lambda: self.fakers["fr_FR"].ssn(),
        }[kind]
        return kind, f()

    def ip(self) -> str:
        o = lambda: str(self.r.randint(1, 254))
        if self.r.random() < 0.8:
            return self.r.choice([f"10.{o()}.{o()}.{o()}", f"172.{self.r.randint(16, 31)}.{o()}.{o()}", f"192.168.{o()}.{o()}"])
        return self.fakers["en_US"].ipv6()

    def hostname(self, fk: Faker) -> str:
        return f"{self.r.choice(['db', 'api', 'build', 'vault', 'grafana', 'k8s-node'])}-{self.r.randint(1, 40)}.{self.r.choice(['internal', 'corp', 'prod'])}.{self.domain(fk)}"

    def secret(self) -> str:
        a = string.ascii_letters + string.digits
        rnd = lambda n: "".join(self.r.choice(a) for _ in range(n))
        return self.r.choice([
            lambda: "sk-proj-" + rnd(48),
            lambda: "sk-ant-api03-" + rnd(64),
            lambda: "ghp_" + rnd(36),
            lambda: "AKIA" + "".join(self.r.choice(string.ascii_uppercase + string.digits) for _ in range(16)),
            lambda: "xoxb-" + "-".join(str(self.r.randint(10**11, 10**12)) for _ in range(2)) + "-" + rnd(24),
            lambda: "glpat-" + rnd(20),
            lambda: "eyJhbGciOiJIUzI1NiJ9." + rnd(40) + "." + rnd(43),
        ])()

    def username(self, fk: Faker) -> str:
        return fk.user_name()

    def dob(self, fk: Faker) -> str:
        d = fk.date_of_birth(minimum_age=20, maximum_age=70)
        return self.r.choice([d.strftime("%Y-%m-%d"), d.strftime("%d/%m/%Y"), d.strftime("%B %d, %Y")])

    # ----------------------------------------------------------------- scenarios

    def chat_request(self, loc: str) -> Doc:
        fk = self.f(loc)
        name = self.person(fk)
        d = Doc()
        v = self.r.randrange(4)
        if v == 0:
            d.t("Can you draft a polite reply to ").e("PERSON", name).t(" (").e("EMAIL", self.email(fk, name)).t(") about the delayed delivery to ").e("ADDRESS", self.address(fk)).t("? Keep it short.")
        elif v == 1:
            d.t("Summarise this for my manager: ").e("PERSON", name).t(" from ").e("ORG", self.org(fk)).t(" called from ").e("PHONE", self.phone(fk)).t(" asking to move the ").p(self.r.choice(PUBLIC)).t(" migration to next quarter.")
        elif v == 2:
            d.t("Email ").e("EMAIL", self.email(fk)).t(" that the contract with ").e("ORG", self.org(fk)).t(" is signed, and cc ").e("PERSON", name).t(".")
        else:
            d.t("Translate to English: our client ").e("PERSON", name).t(" lives at ").e("ADDRESS", self.address(fk)).t(" and prefers calls on ").e("PHONE", self.phone(fk)).t(".")
        return d

    def signature(self, loc: str) -> Doc:
        fk = self.f(loc)
        name = self.person(fk)
        d = Doc().t("Please fix the formatting of this signature:\n\n--\n").e("PERSON", name).t("\n").t(fk.job().split(",")[0]).t("\n").e("ORG", self.org(fk)).t("\n").e("ADDRESS", self.address(fk)).t("\nTel. ").e("PHONE", self.phone(fk)).t("\n").e("EMAIL", self.email(fk, name)).t("\n")
        return d

    def support_ticket(self, loc: str) -> Doc:
        fk = self.f(loc)
        name = self.person(fk)
        d = Doc().t("Ticket #").t(str(self.r.randint(10000, 99999))).t(": customer ").e("PERSON", name).t(" <").e("EMAIL", self.email(fk, name)).t("> says card ").e("CREDIT_CARD", self.card(fk)).t(" was charged twice on ").p(self.r.choice(["Stripe", "PayPal" if False else "Stripe"])).t(". Their IBAN for the refund is ").e("IBAN", self.iban(fk)).t(". Write the refund confirmation.")
        return d

    def hr_record(self, loc: str) -> Doc:
        fk = self.f(loc)
        kind, nid = self.national_id()
        d = Doc().t("Check this onboarding record for missing fields:\nName: ").e("PERSON", self.person(fk)).t("\nDate of birth: ").e("DATE_OF_BIRTH", self.dob(fk)).t(f"\n{kind}: ").e("NATIONAL_ID", nid).t("\nHome address: ").e("ADDRESS", self.address(fk)).t("\nMobile: ").e("PHONE", self.phone(fk)).t("\n")
        return d

    def payment(self, loc: str) -> Doc:
        fk = self.f(loc)
        d = Doc().t(f"Prepare a payment instruction: transfer {self.r.randint(200, 90000)} EUR to ").e("PERSON", self.person(fk)).t(", IBAN ").e("IBAN", self.iban(fk)).t(", reference invoice ").t(str(self.r.randint(1000, 9999))).t(" from ").e("ORG", self.org(fk)).t(".")
        return d

    def devops(self, loc: str) -> Doc:
        fk = self.f(loc)
        user = self.username(fk)
        host = self.hostname(fk)
        d = Doc().t("Why does this fail?\n$ ssh ").e("USERNAME", user).t("@").e("HOSTNAME", host).t("\nssh: connect to host ").e("IP", self.ip()).t(" port 22: Connection refused\n$ cat /Users/").e("USERNAME", user).t("/.config/deploy.env\nAPI_KEY=").e("SECRET", self.secret()).t("\nGRAFANA_URL=").e("URL", f"https://{self.hostname(fk)}/d/ops").t("\nWe run it on ").p(self.r.choice(["Kubernetes", "Linux"])).t(".")
        return d

    def code(self, loc: str) -> Doc:
        fk = self.f(loc)
        d = Doc().t("Refactor this into a function:\n```python\nimport requests\nclient = OpenAI(api_key=\"").e("SECRET", self.secret()).t("\")\nresp = requests.get(\"").e("URL", f"https://{self.hostname(fk)}/v1/users").t("\", auth=(\"").e("USERNAME", self.username(fk)).t("\", \"").e("SECRET", "".join(self.r.choice(string.ascii_letters + string.digits + "!@#$") for _ in range(14))).t("\"))\nnotify(\"").e("EMAIL", self.email(fk)).t("\")\n```\n")
        return d

    def csv(self, loc: str) -> Doc:
        d = Doc().t("Build a churn report from these customers:\nname,email,phone,city,plan\n")
        for _ in range(self.r.randint(4, 9)):
            fk = self.f()
            name = self.person(fk)
            d.e("PERSON", name).t(",").e("EMAIL", self.email(fk, name)).t(",").e("PHONE", self.phone(fk).replace(",", " ")).t(",").t(fk.city()).t(",").t(self.r.choice(["free", "pro", "enterprise"])).t("\n")
        return d

    def meeting(self, loc: str) -> Doc:
        fk = self.f(loc)
        a, b, c = self.person(fk), self.person(fk), self.person(fk)
        d = Doc().t("Turn these notes into action items:\n- ").e("PERSON", a).t(" will send the ").p(self.r.choice(["ISO 27001", "GDPR"])).t(" evidence to ").e("PERSON", b).t(" (").e("EMAIL", self.email(fk, b)).t(")\n- ").e("PERSON", c).t(" to renegotiate with ").e("ORG", self.org(fk)).t(" before the ").p(self.r.choice(["London", "Berlin", "Tokyo"])).t(" offsite\n- Compare ").p("Microsoft").t(" and ").p("Google").t(" pricing.\n")
        return d

    def multilingual(self, loc: str) -> Doc:
        lang = self.r.choice(["de", "fr", "es"])
        fk = self.f({"de": "de_DE", "fr": "fr_FR", "es": "es_ES"}[lang])
        name = self.person(fk)
        d = Doc()
        if lang == "de":
            d.t("Schreib eine kurze Antwort an ").e("PERSON", name).t(" (").e("EMAIL", self.email(fk, name)).t("). Die Lieferung an ").e("ADDRESS", self.address(fk)).t(" verzögert sich; Rückfragen bitte an ").e("PHONE", self.phone(fk)).t(".")
        elif lang == "fr":
            d.t("Rédige un courriel pour ").e("PERSON", name).t(" de ").e("ORG", self.org(fk)).t(" : le virement sur l'IBAN ").e("IBAN", self.iban(fk)).t(" est confirmé. Contact : ").e("EMAIL", self.email(fk, name)).t(".")
        else:
            d.t("Resume el caso de ").e("PERSON", name).t(", con domicilio en ").e("ADDRESS", self.address(fk)).t(" y teléfono ").e("PHONE", self.phone(fk)).t(", para el equipo de ").p("Salesforce").t(".")
        return d

    def negatives(self, loc: str) -> Doc:
        d = Doc().t("Compare ").p("Microsoft").t(" Azure and ").p("Google").t(" Cloud for a team in ").p(self.r.choice(["Paris", "London", "New York"])).t(". What did ").p(self.r.choice(["Satya Nadella", "Tim Cook"])).t(" say about AI at the last keynote? Our stack is ").p("PostgreSQL").t(", ").p("Python").t(" and ").p("Kubernetes").t("; we need ").p("GDPR").t(" compliance. Keep variable names like user_id and email_address.")
        return d

    SCENARIOS = ["chat_request", "signature", "support_ticket", "hr_record", "payment", "devops", "code", "csv", "meeting", "multilingual", "negatives"]
    WEIGHTS = [16, 8, 10, 10, 8, 10, 8, 8, 8, 10, 4]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=1500)
    ap.add_argument("--seed", type=int, default=20261031)
    ap.add_argument("--out", default="synthetic-v1.jsonl")
    a = ap.parse_args()
    g = Gen(a.seed)
    with open(a.out, "w", encoding="utf-8") as out:
        for i in range(a.n):
            sc = g.r.choices(Gen.SCENARIOS, weights=Gen.WEIGHTS)[0]
            loc = g.r.choice(LOCALES)
            d = getattr(g, sc)(loc)
            text = d.text()
            for s in d.spans + d.public:
                assert text[s["start"] : s["end"]] == s["value"], (sc, s)
            out.write(json.dumps({"id": f"s{i:05d}", "scenario": sc, "locale": loc, "text": text, "spans": d.spans, "public": d.public}, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
