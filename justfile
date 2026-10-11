# Recettes communes à l'humain, à Claude et à la CI (7.5, 7.6).
# Sous Windows, les recettes s'exécutent dans PowerShell (15.4) : rester sur
# des commandes valables dans PowerShell et dans sh ; tout script partagé est en Node.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Liste les recettes.
default:
    @just --list

# Tests rapides (Rust).
t:
    cargo nextest run --workspace

# Boucle courte : relance les tests à chaque sauvegarde.
tdd:
    bacon nextest

# Doctests (nextest ne les exécute pas).
doc:
    cargo test --workspace --doc

# Relire les snapshots insta.
snap:
    cargo insta review

# Couverture HTML locale.
cov:
    cargo llvm-cov nextest --workspace --html

# Tout ce que la CI exigera.
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace
    cargo test --workspace --doc
    just web-check

# Serveur Rust sur :8080 (mode production, base locale cra.db).
dev-api:
    cargo run -p cra-server

# Serveur Rust en mode démo, date figée au 11/10/2026.
dev-demo:
    node scripts/run-demo.mjs

# Vite sur :5173, proxy /api vers :8080 (deuxième terminal).
dev-web:
    npm --prefix web run dev

# Tout ce que la CI exige côté front.
web-check:
    npm --prefix web run lint
    npm --prefix web run format:check
    npm --prefix web run typecheck
    npm --prefix web run test

# Régénère openapi.json (Rust) puis schema.d.ts (TypeScript).
gen-api:
    cargo run -q -p cra-server -- openapi --out openapi.json
    npm --prefix web run gen:api

# Échoue si le contrat n'est pas à jour : avant commit, et en CI.
drift:
    just gen-api
    git diff --exit-code -- openapi.json web/src/api/schema.d.ts

# Build complet : front puis binaire release qui l'embarque.
build:
    npm --prefix web run build
    cargo build --release -p cra-server
