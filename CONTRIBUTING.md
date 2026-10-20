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
- Every merge into `main` publishes a new release automatically, so `main` must always build.

## Development

```bash
rustup update            # Rust 1.95 or newer
cargo run -- gui --demo  # try the interface without a pad
cargo test
```

The code and its comments are mostly in Italian; English is fine in issues and pull requests.

## Never commit personal data

No API keys, no `ai.toml`, `settings.conf`, `app_uuid.bin`, `peripheral_id`, and nothing from
`library/`, `documents/`, `chats/`, `exports/`, `signatures/` or `captures/`. The `.gitignore`
already excludes them: please don't force-add them.

## Licence

By contributing you agree that your contribution is licensed under the
[GPL-3.0-or-later](LICENSE), like the rest of the project.
