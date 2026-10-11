# Solution CRA Powered by Rust

Application personnelle de suivi hebdomadaire des fiches CRA (compte rendu
d'activité) d'une alternance, entreprise et formation, du 01/09/2026 au
31/08/2028 : créer et remplir les fiches, suivre les retards, justifier une
semaine, exporter (OneNote, Word, PDF, CSV, JSON) et consulter le bilan.

Back-end Rust (architecture hexagonale, axum, SQLite), front React +
TypeScript embarqué dans le binaire, un seul conteneur.

## Démarrer

Prérequis (Windows) : Rust (rustup, chaîne MSVC), Node 24, `just`,
`cargo-nextest`.

```bash
npm --prefix web ci
```

```bash
just dev-demo
```

Dans un second terminal :

```bash
just dev-web
```

Ouvrir http://localhost:5173. Le mode démo fige la date au 11/10/2026 et sert
un jeu de données fictif ; `just dev-api` lance le mode production sur une
base SQLite locale (`cra.db`).

## Vérifier

```bash
just check
```

```bash
just drift
```

## Organisation

| Dossier | Rôle |
|---|---|
| `crates/domain` | règles métier pures : semaines, plan, statuts, fiches, justifications, bilan |
| `crates/application` | cas d'usage et ports (dépôt, horloge, identifiants, exports) |
| `crates/adapter-*` | mémoire et démo, SQLite, exports, HTTP |
| `bin/cra-server` | assemblage, sous-commandes `serve`, `healthcheck`, `openapi` |
| `web/` | front React (Vite, TanStack Query, Tailwind, Radix) |
| `migrations/` | schéma SQLite |
| `deploy/` | Compose et Caddy pour la VM Oracle |

Les conventions de travail sont dans `CLAUDE.md`.
