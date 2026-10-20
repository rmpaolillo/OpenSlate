# Contributing to OpenSlate

Thanks for your interest! OpenSlate is a personal, vibe-coded project that is shared in case it is
useful to other Bamboo Slate owners. Bug reports, ideas and pull requests are welcome.

## How changes get in

- The project is owned and maintained by [@rmpaolillo](https://github.com/rmpaolillo).
  **Every pull request must be reviewed and approved by the owner before it is merged**
  (see [`.github/CODEOWNERS`](.github/CODEOWNERS)). Nobody else can push to `main`.
- Open an issue first for larger changes, so we can agree on the direction.
- Keep pull requests small and focused. CI (`cargo build` + `cargo test` on Linux and macOS)
  must be green.
- Releases are published by the owner when a set of changes is ready; `main` must always build.

## Development

```bash
rustup update            # Rust 1.95 or newer
cargo run -- gui --demo  # try the interface without a pad
cargo test
```

The code and its comments are mostly in Italian; English is fine in issues and pull requests.

## Translations

The interface text is written in Italian in the code, and that text is the key. Each language
has a table in `src/i18n/<code>.json` (`en`, `es`, `fr`, `de`, `uk`, `zh`, `ja`, `ko`) that maps
the Italian text to the translation; English is the fallback for anything missing. Placeholders
such as `{}` or `{n}` must stay the same, and `cargo test` checks that every table has the same
keys and placeholders as `en.json`.

- **New or changed text**: add the Italian string as a key to every table.
- **Finding untranslated text**: run with `OPENSLATE_I18N_MISSING=missing.txt cargo run -- gui`,
  set the language in File → Options, and look at the strings collected in `missing.txt`.
- **A new language**: add it to `Lang` in `src/i18n.rs` and to `I18N_DATE` in `src/web/i18n.js`.

## Never commit personal data

No API keys, no `ai.toml`, `settings.conf`, `app_uuid.bin`, `peripheral_id`, and nothing from
`library/`, `documents/`, `chats/`, `exports/`, `signatures/` or `captures/`. The `.gitignore`
already excludes them: please don't force-add them.

## Licence

By contributing you agree that your contribution is licensed under the
[GPL-3.0-or-later](LICENSE), like the rest of the project.
