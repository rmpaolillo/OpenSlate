//! Firma digitale PAdES con la firma remota di Aruba (OTP), sul computer e
//! sul telefono.
//!
//! - Il PDF, la `/ByteRange` e la busta CMS li scrive PdfCraft (`pdfcraft-sign`),
//!   come per ogni altra firma digitale: il file non lascia il computer.
//! - Ad Aruba va solo l'impronta (SHA-256) degli attributi firmati, con
//!   l'operazione `signhash` del suo servizio ARSS (SOAP); torna la firma RSA,
//!   che si controlla col certificato dell'utente prima di metterla nel PDF.
//! - Il certificato (e il suo identificativo) si legge con `listCert`, che
//!   chiede solo la password: niente OTP sprecati per controllare l'account.
//!
//! Password e codici OTP non si salvano e non finiscono nei messaggi d'errore.
//! Quando la firma remota entrerà in PdfCraft (storytold/pdfcraft#4045), questo
//! modulo potrà lasciare il posto alla sua.

use anyhow::{anyhow, bail, Context, Result};
use pdfcraft_sign::{Certificate, DigestAlg, DigitalId, PrivateKey};
use std::sync::{Arc, Mutex};

/// Il servizio di firma remota di Aruba.
const ENDPOINT: &str = "https://arss.arubapec.it/ArubaSignService/ArubaSignService";
const NS: &str = "http://arubasignservice.arubapec.it/";
/// Il tipo di dispositivo delle firme remote di Aruba.
const HSM: &str = "COSIGN";

/// L'account di firma remota: `utente@dominio` come nel SelfCare di Aruba
/// (per una firma comprata su pec.it il dominio è `firma`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Account {
    pub user: String,
    pub domain: String,
}

impl Account {
    /// Da `utente@dominio` (o solo `utente`: dominio `firma`).
    pub fn parse(s: &str) -> Account {
        let s = s.trim();
        match s.rsplit_once('@') {
            Some((u, d)) if !u.is_empty() && !d.is_empty() => Account { user: u.trim().into(), domain: d.trim().into() },
            _ => Account { user: s.into(), domain: "firma".into() },
        }
    }

    pub fn label(&self) -> String {
        format!("{}@{}", self.user, self.domain)
    }
}

/// Un certificato di firma dell'account.
#[derive(Clone, Debug)]
pub struct RemoteCert {
    /// L'identificativo per `signhash` (`NEW_2025:…-AS0`).
    pub id: String,
    /// Il certificato (DER).
    pub der: Vec<u8>,
    /// Il titolare (dal certificato).
    pub holder: String,
}

/// Chi porta le richieste ad Aruba: HTTPS nel programma, finto nelle prove.
pub trait Transport: Send + Sync {
    /// Manda l'envelope SOAP e ritorna la risposta (anche quella di un errore SOAP).
    fn post(&self, body: &str) -> Result<String>;
}

/// HTTPS verso Aruba (ureq, come la chat AI).
pub struct Https;

impl Transport for Https {
    fn post(&self, body: &str) -> Result<String> {
        let r = ureq::post(ENDPOINT)
            .timeout(std::time::Duration::from_secs(90))
            .set("Content-Type", "text/xml; charset=utf-8")
            .set("SOAPAction", "\"\"")
            .send_string(body);
        match r {
            Ok(resp) => Ok(resp.into_string()?),
            // un SOAP Fault arriva con HTTP 500: la risposta dice perché
            Err(ureq::Error::Status(_, resp)) => Ok(resp.into_string()?),
            Err(e) => Err(anyhow!("Aruba non risponde ({e}): controlla la connessione")),
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn element(name: &str, value: &str) -> String {
    format!("<{name}>{}</{name}>", escape(value))
}

fn envelope(op: &str, children: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:arss=\"{NS}\"><soapenv:Body><arss:{op}>{children}</arss:{op}></soapenv:Body></soapenv:Envelope>"
    )
}

/// Le credenziali (`auth` dello schema, i campi in ordine): `tag` è
/// `Identity` per `listCert`, `identity` per `signhash`.
fn identity(tag: &str, acc: &Account, password: &str, otp: Option<&str>) -> String {
    let mut s = String::new();
    if let Some(otp) = otp {
        s.push_str(&element("otpPwd", otp));
    }
    s.push_str(&element("typeHSM", HSM));
    s.push_str(&element("typeOtpAuth", &acc.domain));
    s.push_str(&element("user", &acc.user));
    s.push_str(&element("userPWD", password));
    format!("<{tag}>{s}</{tag}>")
}

/// Toglie i segreti da un testo (un messaggio di Aruba potrebbe ripeterli).
fn redact(s: &str, secrets: &[&str]) -> String {
    let mut s = s.to_string();
    for x in secrets.iter().filter(|x| x.len() >= 3) {
        s = s.replace(*x, "•••");
    }
    s
}

/// Il figlio `name` (senza prefisso) di `node`.
fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children().find(|c| c.tag_name().name() == name).and_then(|c| c.text())
}

/// Il messaggio per l'utente da `status`/`return_code`/`description` di Aruba.
fn refusal(ret: roxmltree::Node, what: &str, secrets: &[&str]) -> anyhow::Error {
    let code = child(ret, "return_code").unwrap_or("").trim();
    let desc = redact(child(ret, "description").unwrap_or("").trim(), secrets);
    let why = match code {
        "0004" => "codice OTP sbagliato, già usato o scaduto (o OTP bloccato: in quel caso sbloccalo dal SelfCare di Aruba)".to_string(),
        "0003" => "utente, dominio o password sbagliati".to_string(),
        "0001" => format!("Aruba dice che la richiesta non è valida: {desc}"),
        _ if desc.is_empty() => format!("errore {code}"),
        _ if code.is_empty() => desc,
        _ => format!("{desc} (errore {code})"),
    };
    anyhow!("{what}: {why}")
}

/// Chiama `op` e ritorna il testo della risposta, già controllato (non un SOAP Fault).
fn call(t: &dyn Transport, op: &str, children: &str, secrets: &[&str]) -> Result<String> {
    let text = t.post(&envelope(op, children))?;
    let doc = roxmltree::Document::parse(&text).map_err(|_| anyhow!("Aruba ha risposto in un modo che non capisco"))?;
    if let Some(f) = doc.descendants().find(|n| n.tag_name().name() == "Fault") {
        let msg = f.descendants().find(|n| n.tag_name().name() == "faultstring").and_then(|n| n.text()).unwrap_or("errore");
        bail!("Aruba: {}", redact(msg, secrets));
    }
    Ok(text)
}

/// L'elemento `return` della risposta.
fn returned<'a>(doc: &'a roxmltree::Document<'a>) -> Result<roxmltree::Node<'a, 'a>> {
    doc.descendants().find(|n| n.tag_name().name() == "return").ok_or_else(|| anyhow!("Aruba ha risposto senza risultato"))
}

fn b64(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD.decode(clean).ok()
}

/// I certificati di firma dell'account (solo password: nessun OTP).
pub fn certificates(t: &dyn Transport, acc: &Account, password: &str) -> Result<Vec<RemoteCert>> {
    let secrets = [password];
    let text = call(t, "listCert", &identity("Identity", acc, password, None), &secrets)?;
    let doc = roxmltree::Document::parse(&text)?;
    let ret = returned(&doc)?;
    if !child(ret, "status").unwrap_or("").trim().eq_ignore_ascii_case("OK") {
        return Err(refusal(ret, "Aruba non ha dato i certificati", &secrets));
    }
    let mut out = Vec::new();
    // prima `app1` (i certificati di firma), poi `app2`
    for list in ["app1", "app2"] {
        for c in ret.children().filter(|c| c.tag_name().name() == list) {
            let (Some(id), Some(der)) = (child(c, "id"), child(c, "value").and_then(b64)) else { continue };
            let Ok(cert) = Certificate::parse(&der) else { continue };
            let holder = cert.subject.common_name().map(String::from).unwrap_or_else(|| cert.subject.display_name());
            out.push(RemoteCert { id: id.trim().to_string(), der, holder });
        }
    }
    if out.is_empty() {
        bail!("nell'account {} non c'è un certificato di firma", acc.label());
    }
    Ok(out)
}

/// Chi firma l'impronta: Aruba, o uno finto nelle prove. `msg` è quello che
/// PdfCraft fa firmare (gli attributi firmati): ad Aruba va solo `digest`.
trait HashSigner: Send + Sync {
    fn sign(&self, alg: DigestAlg, msg: &[u8], digest: &[u8]) -> Result<Vec<u8>>;
}

/// `signhash` di Aruba, con l'OTP (buono per una firma sola).
struct Arss {
    t: Arc<dyn Transport>,
    acc: Account,
    password: String,
    otp: String,
    cert_id: String,
}

impl HashSigner for Arss {
    fn sign(&self, alg: DigestAlg, _msg: &[u8], digest: &[u8]) -> Result<Vec<u8>> {
        use base64::Engine;
        let hashtype = match alg {
            DigestAlg::Sha256 => "SHA256",
            DigestAlg::Sha384 => "SHA384",
            DigestAlg::Sha512 => "SHA512",
            other => bail!("impronta {} non usata da Aruba", other.name()),
        };
        let secrets = [self.password.as_str(), self.otp.as_str()];
        let req = [
            element("certID", &self.cert_id),
            element("hash", &base64::engine::general_purpose::STANDARD.encode(digest)),
            element("hashtype", hashtype),
            identity("identity", &self.acc, &self.password, Some(&self.otp)),
            element("requirecert", "false"),
        ]
        .concat();
        let text = call(&*self.t, "signhash", &format!("<SignHashRequest>{req}</SignHashRequest>"), &secrets)?;
        let doc = roxmltree::Document::parse(&text)?;
        let ret = returned(&doc)?;
        let status = child(ret, "status").unwrap_or("").trim();
        match child(ret, "signature").filter(|s| !s.trim().is_empty()) {
            Some(sig) if !status.eq_ignore_ascii_case("KO") => b64(sig).ok_or_else(|| anyhow!("Aruba ha dato una firma illeggibile")),
            _ => Err(refusal(ret, "Aruba non ha firmato", &secrets)),
        }
    }
}

/// La chiave per PdfCraft: chiede la firma a `signer` e la controlla col
/// certificato prima di darla (una firma sbagliata non entra mai nel PDF).
struct RemoteKey {
    signer: Box<dyn HashSigner>,
    public: pdfcraft_sign::PublicKey,
    /// L'errore vero (PdfCraft riceve solo «firma remota non riuscita»).
    error: Arc<Mutex<Option<anyhow::Error>>>,
}

impl RemoteKey {
    fn checked(&self, alg: DigestAlg, msg: &[u8]) -> Result<Vec<u8>> {
        use pdfcraft_sign::keys::Scheme;
        use pdfcraft_sign::PublicKey;
        let digest = alg.digest(&[msg]);
        let sig = self.signer.sign(alg, msg, &digest)?;
        let scheme = match &self.public {
            PublicKey::Rsa { .. } => Scheme::RsaPkcs1,
            PublicKey::P256(_) | PublicKey::P384(_) => Scheme::Ecdsa,
            _ => bail!("il certificato ha una chiave che OpenSlate non sa usare"),
        };
        if self.public.verify(scheme, alg, &digest, &sig).unwrap_or(false) {
            return Ok(sig);
        }
        bail!("la firma arrivata da Aruba non corrisponde al tuo certificato: il documento non è stato firmato")
    }
}

impl pdfcraft_sign::keys::ExternalKey for RemoteKey {
    fn sign(&self, alg: DigestAlg, msg: &[u8]) -> Result<Vec<u8>, pdfcraft_sign::SignError> {
        self.checked(alg, msg).map_err(|e| {
            let text = e.to_string();
            *self.error.lock().unwrap() = Some(e);
            pdfcraft_sign::SignError::Crypto(text)
        })
    }
}

/// La data della firma come la vuole il PDF (`D:AAAAMMGGhhmmss+hh'mm'`).
fn pdf_date(now: chrono::DateTime<chrono::Local>) -> String {
    let off = now.offset().local_minus_utc();
    let (sign, off) = if off < 0 { ('-', -off) } else { ('+', off) };
    format!("{}{sign}{:02}'{:02}'", now.format("D:%Y%m%d%H%M%S"), off / 3600, off % 3600 / 60)
}

/// Firma `pdf` (PAdES, invisibile: la firma grafica e il timbro ci sono già)
/// con `signer` e il certificato `cert`. Il PDF firmato comincia con i byte
/// di quello dato: la firma si aggiunge in fondo, il documento resta com'era.
fn sign_with(pdf: &[u8], cert: &RemoteCert, signer: Box<dyn HashSigner>) -> Result<Vec<u8>> {
    let certificate = Certificate::parse(&cert.der).map_err(|e| anyhow!("il certificato di Aruba non si legge: {e}"))?;
    let error = Arc::new(Mutex::new(None));
    let key = RemoteKey { signer, public: certificate.public_key.clone(), error: error.clone() };
    let id = DigitalId { key: PrivateKey::external(certificate.public_key.clone(), Arc::new(key)), certificate, chain: Vec::new(), friendly_name: None };
    let doc = pdfcraft_cos::Document::open(Arc::new(pdf.to_vec())).map_err(|e| anyhow!("il PDF non si apre: {e}"))?;
    let opts = pdfcraft_sign::pdf::SignOptions { date: pdf_date(chrono::Local::now()), ..Default::default() };
    let out = pdfcraft_sign::pdf::sign(&doc, &id, &opts).map_err(|e| error.lock().unwrap().take().unwrap_or_else(|| anyhow!("firma del PDF: {e}")))?;
    if !out.starts_with(pdf) {
        bail!("il PDF firmato non contiene più il documento com'era: non lo salvo");
    }
    Ok(out)
}

/// Firma `pdf` con la firma remota di Aruba: una firma, un codice OTP.
pub fn sign_pdf(t: Arc<dyn Transport>, pdf: &[u8], acc: &Account, password: &str, otp: &str, cert: &RemoteCert) -> Result<Vec<u8>> {
    let otp: String = otp.chars().filter(|c| !c.is_whitespace()).collect();
    if otp.len() < 6 || otp.len() > 8 || !otp.chars().all(|c| c.is_ascii_digit()) {
        bail!("il codice OTP sono 6–8 cifre (quelle dell'app Aruba OTP)");
    }
    let signer = Arss { t, acc: acc.clone(), password: password.to_string(), otp, cert_id: cert.id.clone() };
    sign_with(pdf, cert, Box::new(signer)).context("firma digitale")
}

/// Dove scrivere il PDF firmato digitalmente: accanto all'originale,
/// `nome-firmato-digitalmente.pdf` (o con un numero se c'è già).
pub fn signed_path(src: &std::path::Path) -> std::path::PathBuf {
    let dir = src.parent().unwrap_or(std::path::Path::new("."));
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("documento");
    let mut p = dir.join(format!("{stem}-firmato-digitalmente.pdf"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{stem}-firmato-digitalmente-{n}.pdf"));
        n += 1;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un certificato e la sua chiave, fatti qui.
    fn identity_pair() -> (RemoteCert, PrivateKey) {
        let key = PrivateKey::generate_rsa(2048).unwrap();
        let name = pdfcraft_sign::Name::build("Mario Rossi", "", "", "", "IT");
        let from = pdfcraft_sign::Time { year: 2025, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        let cert = Certificate::self_signed(&name, &key, from, 3, &[7]).unwrap();
        (RemoteCert { id: "NEW_2025:abc-AS0".into(), der: cert.raw.clone(), holder: "Mario Rossi".into() }, key)
    }

    /// Un Aruba finto: firma con la chiave vera (e ricorda che cosa gli è arrivato).
    struct Fake {
        key: PrivateKey,
        got: Arc<Mutex<Vec<Vec<u8>>>>,
        wrong: bool,
    }

    impl HashSigner for Fake {
        fn sign(&self, alg: DigestAlg, msg: &[u8], digest: &[u8]) -> Result<Vec<u8>> {
            self.got.lock().unwrap().push(digest.to_vec());
            let m: &[u8] = if self.wrong { b"altro" } else { msg };
            Ok(self.key.sign(alg, m)?)
        }
    }

    fn pdf() -> Vec<u8> {
        let tmp = std::env::temp_dir().join(format!("openslate-fd-{}.pdf", std::process::id()));
        crate::pdfdoc::image_pdf(&tmp, 2, 2, &[255; 12], 595.0, 842.0).unwrap();
        std::fs::read(tmp).unwrap()
    }

    #[test]
    fn firma_e_si_verifica() {
        let (cert, key) = identity_pair();
        let got = Arc::new(Mutex::new(Vec::new()));
        let src = pdf();
        let out = sign_with(&src, &cert, Box::new(Fake { key, got: got.clone(), wrong: false })).unwrap();
        assert!(out.starts_with(&src) && out.len() > src.len());
        // ad Aruba è andata una sola impronta SHA-256, non il documento
        let got = got.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 32);
        let doc = pdfcraft_cos::Document::open(Arc::new(out.clone())).unwrap();
        let trust = pdfcraft_sign::pdf::TrustStore { certs: vec![Certificate::parse(&cert.der).unwrap()] };
        let sigs = pdfcraft_sign::pdf::list(&doc, &out, &trust);
        assert_eq!(sigs.len(), 1);
        let s = &sigs[0];
        assert!(s.signed && s.sub_filter.as_deref() == Some("ETSI.CAdES.detached"), "{s:?}");
        assert_eq!(s.status, pdfcraft_sign::pdf::Status::Valid, "{:?}", s.details);
    }

    #[test]
    fn firma_sbagliata_non_entra() {
        let (cert, key) = identity_pair();
        let err = sign_with(&pdf(), &cert, Box::new(Fake { key, got: Default::default(), wrong: true })).unwrap_err();
        assert!(err.to_string().contains("non corrisponde"), "{err:#}");
    }

    struct Canned(String, Mutex<Vec<String>>);
    impl Transport for Canned {
        fn post(&self, body: &str) -> Result<String> {
            self.1.lock().unwrap().push(body.to_string());
            Ok(self.0.clone())
        }
    }

    fn soap(inner: &str) -> String {
        format!("<S:Envelope xmlns:S=\"http://schemas.xmlsoap.org/soap/envelope/\"><S:Body><ns2:r xmlns:ns2=\"{NS}\"><return>{inner}</return></ns2:r></S:Body></S:Envelope>")
    }

    #[test]
    fn certificati_e_credenziali() {
        use base64::Engine;
        let (cert, _) = identity_pair();
        let v = base64::engine::general_purpose::STANDARD.encode(&cert.der);
        let t = Canned(soap(&format!("<app1><id>NEW_2025:abc-AS0</id><value>{v}</value></app1><status>OK</status>")), Default::default());
        let acc = Account::parse("mario@firma");
        let list = certificates(&t, &acc, "p<w&").unwrap();
        assert_eq!((list[0].id.as_str(), list[0].holder.as_str()), ("NEW_2025:abc-AS0", "Mario Rossi"));
        let sent = &t.1.lock().unwrap()[0];
        assert!(sent.contains("<arss:listCert><Identity><typeHSM>COSIGN</typeHSM><typeOtpAuth>firma</typeOtpAuth><user>mario</user><userPWD>p&lt;w&amp;</userPWD></Identity>"), "{sent}");
        assert!(!sent.contains("otpPwd"));
    }

    #[test]
    fn otp_sbagliato_in_parole_senza_segreti() {
        let t = Arc::new(Canned(soap("<description>OTP 12345678 non valido</description><return_code>0004</return_code><status>KO</status>"), Default::default()));
        let (cert, _) = identity_pair();
        let s = Arss { t: t.clone(), acc: Account::parse("mario"), password: "segreta".into(), otp: "12345678".into(), cert_id: cert.id.clone() };
        let e = s.sign(DigestAlg::Sha256, b"x", &[0; 32]).unwrap_err().to_string();
        assert!(e.contains("OTP sbagliato") && !e.contains("12345678") && !e.contains("segreta"), "{e}");
        let sent = &t.1.lock().unwrap()[0];
        assert!(sent.contains("<SignHashRequest><certID>NEW_2025:abc-AS0</certID><hash>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</hash><hashtype>SHA256</hashtype><identity><otpPwd>12345678</otpPwd>"), "{sent}");
        assert_eq!(Account::parse("mario").domain, "firma");
    }

    #[test]
    fn otp_solo_cifre() {
        let (cert, _) = identity_pair();
        let t: Arc<dyn Transport> = Arc::new(Canned(String::new(), Default::default()));
        assert!(sign_pdf(t, b"%PDF", &Account::parse("m"), "p", "12ab", &cert).unwrap_err().to_string().contains("6–8 cifre"));
    }
}
