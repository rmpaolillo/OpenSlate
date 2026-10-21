# Pacchetti Typst inclusi nell'app

OpenSlate usa Typst (https://typst.app) senza rete: i pacchetti che servono sono qui e
`src/typstworld.rs` li dà al compilatore come se fossero `@preview/nome:versione`.

| Cartella | Pacchetto | Licenza | Uso |
|---|---|---|---|
| `lilaq/0.6.0` | [Lilaq](https://github.com/lilaq-project/lilaq) (Mc-Zen) | MIT | grafici della chat (`src/charts.rs`) |
| `zero/0.6.1` | [zero](https://github.com/Mc-Zen/zero) (Mc-Zen) | MIT | numeri sugli assi (per Lilaq) |
| `tiptoe/0.4.0` | [tiptoe](https://github.com/Mc-Zen/tiptoe) (Mc-Zen) | MIT | frecce degli assi (per Lilaq) |
| `elembic/1.1.1` | [elembic](https://github.com/PgBiel/elembic) (PgBiel) | MIT OR Apache-2.0 | elementi personalizzati (per Lilaq) |
| `suiji/0.5.1` | [suiji](https://github.com/liuguangxi/suiji) (liuguangxi) | MIT | numeri casuali (per Lilaq; contiene `core.wasm`) |
| `mitex-specs` | [MiTeX](https://github.com/mitex-rs/mitex), cartella `packages/mitex/specs` del tag 0.2.7 | Apache-2.0 | definizioni per il LaTeX convertito (`src/mathtex.rs`, `src/charts.rs`) |

Ogni cartella ha `typst.toml`, `src/` e la licenza del pacchetto, senza modifiche.

`src/typstfiles.rs` è l'elenco dei file inclusi nel programma: dopo aver aggiunto o
aggiornato un pacchetto va rigenerato, dalla cartella del progetto:

```bash
python3 - <<'PY'
import os
D = 'src/typst-pkg/'
out = ["// File generato: i pacchetti Typst inclusi nell'app (vedi src/typst-pkg/README.md).",
       "// Rigenerare con lo script in quel README quando si aggiornano i pacchetti.",
       "pub const FILES: &[(&str, &[u8])] = &["]
for root, _, files in sorted(os.walk(D)):
    for f in sorted(files):
        if f.endswith(('.typ', '.toml', '.wasm')):
            rel = os.path.relpath(os.path.join(root, f), D)
            out.append(f'    ("{rel}", include_bytes!("typst-pkg/{rel}")),')
out.append('];')
open('src/typstfiles.rs', 'w').write('\n'.join(out) + '\n')
PY
```
