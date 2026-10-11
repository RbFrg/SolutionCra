# Solution CRA Powered by Rust

Suivi hebdomadaire des fiches CRA (entreprise et formation) du 01/09/2026 au
31/08/2028. Référence complète : skill `solution-cra-powered-by-rust` (lire
`SKILL.md`, puis seulement la référence utile à la tâche).

## Commandes

| Commande | Rôle |
|---|---|
| `just check` | **validation unique** avant de déclarer une tâche terminée : fmt, clippy `-D warnings`, nextest, doctests, puis lint, format, tsc et Vitest du front |
| `just drift` | régénère `openapi.json` et `web/src/api/schema.d.ts` puis échoue s'il y a une différence |
| `just gen-api` | régénère le contrat (Rust puis TypeScript) |
| `just dev-api` | serveur Rust sur :8080 (production, base `cra.db`) |
| `just dev-demo` | serveur Rust en mode démo (date figée au 11/10/2026, données fictives) |
| `just dev-web` | Vite sur :5173, proxy `/api` et `/health` vers :8080 |
| `cargo nextest run -p <crate>` | boucle courte d'un crate |
| `npm --prefix web run test -- <fichier>` | boucle courte du front |

Sous Windows, `cargo` est dans `%USERPROFILE%\.cargo\bin` ; `just` exécute ses
recettes dans PowerShell. Tout script partagé est en Node (`scripts/*.mjs`).

## Structure et règle de dépendance

```
crates/domain          pur, synchrone, sans I/O ni horloge : semaine, plan, statuts, fiche, justification, bilan
crates/application     cas d'usage + ports (Depot avec unité de travail `Lot`, Clock, IdGenerator, Exporteur)
crates/adapter-memory  dépôt en mémoire + jeu de démo (20.5)
crates/adapter-store   SQLite (sqlx), migrations dans migrations/
crates/adapter-export  HTML OneNote, Word (docx-rs), PDF (générateur maison), CSV, zip
crates/adapter-http    axum : DTO + utoipa, problem+json, CSRF, service du front
bin/cra-server         composition root : APP_MODE, sous-commandes serve | healthcheck | openapi
web/                   React + TypeScript (Vite), ne dépend que de openapi.json
```

`domain` ne dépend de rien d'applicatif ; `application` dépend de `domain` ;
les adaptateurs dépendent de `application` ; seul `bin/cra-server` les connaît
tous. Le front ne dépend que du contrat généré.

## Règles

- **Test d'abord**, rouge avant vert. Nom de test en phrase `snake_case`
  (`dimanche_23h59m59_paris_est_encore_a_faire`) ; citer les identifiants du
  catalogue (SEM1, FIC3, H8…) dans le message de commit.
- **Le temps est un paramètre** : jamais `Utc::now()` dans `domain` ni
  `application` (port `Clock`, `FixedClock` dans les tests). Le statut d'une
  semaine n'est jamais stocké.
- **Le journal est en ajout seul** et s'écrit dans le même `Lot` que le
  changement métier.
- **Aucune règle métier dans le front** : il affiche les statuts, retards et
  libellés renvoyés par le serveur.
- `openapi.json` et `web/src/api/schema.d.ts` sont générés : ne jamais les éditer.
- Pas de `unwrap()` ni `expect()` hors tests ; erreurs `thiserror` dans les
  crates, `anyhow` dans `bin/`.
- Aucune donnée réelle dans le dépôt, les fixtures ou la démo.
- Branche courte et pull request vers `main` (squash) ; Conventional Commits.

## Définition de « terminé »

`just check` vert, `just drift` vide, tests du critère d'acceptation écrits et
verts, aucun `unwrap()` ajouté hors tests, PR avec le gabarit, CI verte,
écrans conformes à la maquette (clair, sombre, bureau, mobile).
