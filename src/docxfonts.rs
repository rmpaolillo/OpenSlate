//! Caratteri dentro un file Word (ECMA-376, parte 1, 17.8.1): l'appunto scritto
//! a mano si vede a mano anche dove i caratteri di OpenSlate non sono
//! installati. WordCraft scrive il documento; qui si aggiungono al file i due
//! caratteri (normale e grassetto) «offuscati» come vuole Word, la loro voce
//! nella tabella dei caratteri e `w:embedTrueTypeFonts` nelle impostazioni.

use anyhow::{bail, Context, Result};
use std::io::{Read, Write};
use std::path::Path;

const FONT_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/font";
const FONT_TABLE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable";
const ODTTF: &str = "application/vnd.openxmlformats-officedocument.obfuscatedFont";
const FONT_TABLE_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml";
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// La chiave (un GUID) e il carattere offuscato: i primi 32 byte in XOR coi
/// 16 byte del GUID letti al contrario.
fn obfuscate(data: &[u8], seed: u64) -> (String, Vec<u8>) {
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut bytes = [0u8; 16];
    for b in bytes.iter_mut() {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = (x >> 56) as u8;
    }
    let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    let guid = format!("{{{}-{}-{}-{}-{}}}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32]);
    let mut key = [0u8; 16];
    for (i, b) in bytes.iter().enumerate() {
        key[15 - i] = *b;
    }
    let mut out = data.to_vec();
    for (i, b) in out.iter_mut().take(32).enumerate() {
        *b ^= key[i % 16];
    }
    (guid, out)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Aggiunge i caratteri `name` (normale e grassetto) al file Word `docx`.
pub fn embed(docx: &Path, name: &str, regular: &[u8], bold: &[u8]) -> Result<()> {
    let raw = std::fs::read(docx).with_context(|| docx.display().to_string())?;
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(&raw)).context("il file Word non si apre")?;
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zin.len() {
        let mut f = zin.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        parts.push((f.name().to_string(), b));
    }
    let get = |parts: &[(String, Vec<u8>)], n: &str| parts.iter().position(|(p, _)| p == n);
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let seed = raw.len() as u64 ^ (name.len() as u64) << 32;
    let (key_r, data_r) = obfuscate(regular, seed);
    let (key_b, data_b) = obfuscate(bold, seed.wrapping_add(1));
    let (file_r, file_b) = ("fonts/openslate-hand-1.odttf", "fonts/openslate-hand-2.odttf");
    let (id_r, id_b) = ("rIdOsHand1", "rIdOsHand2");
    let entry = format!(
        "<w:font w:name=\"{n}\"><w:charset w:val=\"00\"/><w:family w:val=\"script\"/><w:pitch w:val=\"variable\"/>\
         <w:embedRegular r:id=\"{id_r}\" w:fontKey=\"{key_r}\"/><w:embedBold r:id=\"{id_b}\" w:fontKey=\"{key_b}\"/></w:font>",
        n = xml_escape(name)
    );
    // la tabella dei caratteri: la voce di questo carattere al posto di quella che c'è
    match get(&parts, "word/fontTable.xml") {
        Some(i) => {
            let mut t = text(&parts[i].1);
            let open = format!("<w:font w:name=\"{}\"", xml_escape(name));
            if let Some(a) = t.find(&open) {
                let end = t[a..].find("</w:font>").map(|e| a + e + "</w:font>".len()).or_else(|| t[a..].find("/>").map(|e| a + e + 2));
                if let Some(e) = end {
                    t.replace_range(a..e, "");
                }
            }
            if !t.contains("xmlns:r=") {
                t = t.replacen("<w:fonts ", &format!("<w:fonts xmlns:r=\"{R}\" "), 1);
            }
            let Some(close) = t.rfind("</w:fonts>") else { bail!("tabella dei caratteri del Word illeggibile") };
            t.insert_str(close, &entry);
            parts[i].1 = t.into_bytes();
        }
        None => {
            let t = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:fonts xmlns:w=\"{W}\" xmlns:r=\"{R}\">{entry}</w:fonts>");
            parts.push(("word/fontTable.xml".into(), t.into_bytes()));
            // il documento deve sapere che c'è
            let rels = "word/_rels/document.xml.rels";
            let Some(i) = get(&parts, rels) else { bail!("il Word non ha {rels}") };
            let mut r = text(&parts[i].1);
            let Some(close) = r.rfind("</Relationships>") else { bail!("{rels} illeggibile") };
            r.insert_str(close, &format!("<Relationship Id=\"rIdOsFontTable\" Type=\"{FONT_TABLE_REL}\" Target=\"fontTable.xml\"/>"));
            parts[i].1 = r.into_bytes();
            let ct = "[Content_Types].xml";
            let Some(i) = get(&parts, ct) else { bail!("il Word non ha {ct}") };
            let mut c = text(&parts[i].1);
            if !c.contains("/word/fontTable.xml") {
                let Some(close) = c.rfind("</Types>") else { bail!("{ct} illeggibile") };
                c.insert_str(close, &format!("<Override PartName=\"/word/fontTable.xml\" ContentType=\"{FONT_TABLE_TYPE}\"/>"));
                parts[i].1 = c.into_bytes();
            }
        }
    }
    // i collegamenti della tabella ai file dei caratteri
    let frels = "word/_rels/fontTable.xml.rels";
    let links = format!(
        "<Relationship Id=\"{id_r}\" Type=\"{FONT_REL}\" Target=\"{file_r}\"/><Relationship Id=\"{id_b}\" Type=\"{FONT_REL}\" Target=\"{file_b}\"/>"
    );
    match get(&parts, frels) {
        Some(i) => {
            let mut r = text(&parts[i].1);
            for id in [id_r, id_b] {
                if let Some(a) = r.find(&format!("Id=\"{id}\"")) {
                    let s = r[..a].rfind('<').unwrap_or(a);
                    let e = r[a..].find("/>").map(|e| a + e + 2).unwrap_or(a);
                    r.replace_range(s..e, "");
                }
            }
            let Some(close) = r.rfind("</Relationships>") else { bail!("{frels} illeggibile") };
            r.insert_str(close, &links);
            parts[i].1 = r.into_bytes();
        }
        None => parts.push((
            frels.into(),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{links}</Relationships>"
            )
            .into_bytes(),
        )),
    }
    for (f, d) in [(file_r, data_r), (file_b, data_b)] {
        let p = format!("word/{f}");
        parts.retain(|(n, _)| *n != p);
        parts.push((p, d));
    }
    // il tipo dei file .odttf
    let ct = "[Content_Types].xml";
    if let Some(i) = get(&parts, ct) {
        let mut c = text(&parts[i].1);
        if !c.contains("Extension=\"odttf\"") {
            if let Some(close) = c.rfind("</Types>") {
                c.insert_str(close, &format!("<Default Extension=\"odttf\" ContentType=\"{ODTTF}\"/>"));
            }
            parts[i].1 = c.into_bytes();
        }
    }
    // Word: «incorpora i caratteri»
    if let Some(i) = get(&parts, "word/settings.xml") {
        let mut t = text(&parts[i].1);
        if !t.contains("w:embedTrueTypeFonts") {
            if let Some(a) = t.find("<w:settings") {
                if let Some(e) = t[a..].find('>') {
                    t.insert_str(a + e + 1, "<w:embedTrueTypeFonts/>");
                }
            }
            parts[i].1 = t.into_bytes();
        }
    }
    // riscrive il file (prima accanto, poi al suo posto)
    let tmp = docx.with_extension("docx.tmp");
    {
        let mut zout = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        // [Content_Types].xml per primo, come Word
        parts.sort_by_key(|(n, _)| n != ct);
        for (n, d) in &parts {
            zout.start_file(n.as_str(), opts)?;
            zout.write_all(d)?;
        }
        zout.finish()?;
    }
    std::fs::rename(&tmp, docx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiave_come_la_legge_word() {
        let data: Vec<u8> = (0..64u8).collect();
        let (guid, out) = obfuscate(&data, 7);
        assert_eq!(guid.len(), 38);
        // all'indietro: la stessa chiave rimette i byte com'erano
        let digits: Vec<u8> = guid.bytes().filter(|c| !matches!(c, b'{' | b'}' | b'-')).collect();
        let mut key = [0u8; 16];
        for (i, pair) in digits.chunks(2).enumerate() {
            key[15 - i] = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
        }
        let back: Vec<u8> = out.iter().enumerate().map(|(i, b)| if i < 32 { b ^ key[i % 16] } else { *b }).collect();
        assert_eq!(back, data);
        assert_eq!(&out[32..], &data[32..]);
    }
}
