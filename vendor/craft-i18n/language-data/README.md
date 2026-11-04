# Language data

The per-locale data the Crafting Apps check and break text with: spelling word lists and
hyphenation patterns ([ADR 0004](../docs/adr/0004-third-party-language-data.md),
[ADR 0005](../docs/adr/0005-language-data-lives-here.md); terms in [`GLOSSARY.md`](../GLOSSARY.md)).
The code that reads it is `craft-proof` in [craft-libs](https://github.com/storytold/craft-libs)
(`crates/proof`). Grammar rules and suggestion tuning are code, not Language data, and stay there.

## Layout

One directory per locale, `language-data/<locale>/`, named in lowercase BCP 47. A regional locale
(`en-gb`) falls back to its language (`en`) for anything it doesn't have.

| Path | What | Format |
|---|---|---|
| `en-us/`, `en-gb/`, `en-ca/`, `en-au/` `spelling.aff` + `spelling.dic` | SCOWL `rel-2026.02.25` Hunspell dictionaries (size 60): American, British `-ise`, Canadian, Australian English spelling, with regional variants | Hunspell `.aff`/`.dic`, unmodified upstream files renamed |
| `en-us/`, `en-gb/`, `en-ca/`, `en-au/` `LICENCE-SCOWL.txt` | The upstream SCOWL README: copyright and licence notices (SCOWL, Ispell BSD, and Benjamin Titze's notice), for About | text, unmodified |
| `en-us/en-us.dic` | Moby Hyphenator II word list (~165k words with break points): English hyphenation, and the spelling fallback until craft-proof reads SCOWL | WordCraft's compact word list: zlib-compressed, decoded by `craft-proof`'s `dict.rs` (not a Hunspell `.dic`) |
| `en-us/hyphenation.pat` | Our own US English hyphenation patterns, for words not in the list | Liang/TeX patterns |
| `en-us/en-extra.txt` | Common modern English words the Moby list lacks | one lowercase base form per line, `#` comments |
| `en-us/en-variants.txt` | American/British spelling pairs, with a policy per English | `POLICY AMERICAN BRITISH` per line, `#` comments |
| `de/hyphenation.pat`, `pl/hyphenation.pat`, `pt/hyphenation.pat`, `it/hyphenation.pat` | hyph-utf8 TeX hyphenation patterns (`hyph-de-1996.tex`, `hyph-pl.tex`, `hyph-pt.tex`, `hyph-it.tex`) | unmodified `.tex`, compiled when loaded |
| `ru/ru-extra.txt` | Common modern Russian words older Hunspell dictionaries lack, with their declension classes | one base form and class per line, `#` comments |

Everything below the SCOWL rows is the data WordCraft embedded before the move, byte for byte
(hyphenation patterns renamed to `hyphenation.pat`, contents unchanged). It is the carry-over ADR
0004 and ADR 0005 allow: the non-English files leave the default build in the release that ships
Language packs.

**Interim.** The SCOWL `spelling.aff`/`spelling.dic` are the English spelling dictionaries once
craft-proof reads them (craft-libs CP2.2; it checks en-AU as British-spelling English); the Moby
`en-us.dic` stays as the hyphenation word list and the spelling fallback, and `en-extra.txt` and
`en-variants.txt` stay under their names until `craft-proof` no longer needs them. The committed
files' Licence class lives in `ATTRIBUTION.md`. `spelling.aff`, `spelling.dic` and `hyphenation.pat`
are the final names.

## Language packs

Every other locale is a Language pack: `manifest.toml` pins each upstream file (URL at a commit,
sha256, size, SPDX licence, Licence class) and `checks/<locale>.toml` holds the words its
load-and-check test uses, or why the pack is hidden. Nothing in a pack is committed; a release
workflow downloads, checks and publishes them. Format, checks and the release layout:
[`docs/language-data-format.md`](../docs/language-data-format.md).

## Rules

- **Unmodified upstream files.** Third-party files are kept exactly as published; our own
  corrections go in our own files (`*-extra.txt`), under this repository's licence.
- **Every file has a row in [`ATTRIBUTION.md`](../ATTRIBUTION.md)** (Language data section): author,
  source (with the upstream SHA-256 where there is one), licence and Licence class. The checker
  (`crates/craft-i18n-tests/tests/language_data.rs`) fails otherwise, and on any class but
  Embeddable or a copyleft licence that can't be embedded (LGPL, GPL, AGPL, CC-BY-SA): committed
  files are embedded. Dotfiles count. The licence table was signed off on 2026-11-01 (ADR 0004).
- **Licence texts.** `licences/<SPDX id>.txt` are the standard texts (spdx/license-list-data) the
  packs carry; they need no row.
- **Bytes are kept.** `.gitattributes` marks `language-data/**` as `-text`, so no line-ending
  conversion touches the files.

## How the apps use it

Like the catalogs, it is an optional build input: `craft-proof`'s `build.rs` embeds the files it
names from `$CRAFT_I18N_DIR/language-data/`. Without `CRAFT_I18N_DIR` the crate still builds, with no
embedded Language data (it warns); `CRAFT_I18N_REQUIRED=1`, as releases set, makes that an error.
See [`docs/integration.md`](../docs/integration.md) for the variables.
