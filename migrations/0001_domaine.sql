-- Domaine (18.2). Les contraintes sont une seconde ligne de défense : la
-- vérité des règles reste le domaine Rust.

CREATE TABLE mission (
  id          TEXT PRIMARY KEY,                       -- UUID v7
  nom         TEXT NOT NULL CHECK (length(trim(nom)) > 0),
  nom_cle     TEXT NOT NULL UNIQUE,                   -- sans accents ni casse (3.7)
  code        TEXT,
  archivee    INTEGER NOT NULL DEFAULT 0 CHECK (archivee IN (0, 1)),
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
) STRICT;

CREATE TABLE competence (
  id          TEXT PRIMARY KEY,
  nom         TEXT NOT NULL CHECK (length(trim(nom)) > 0),
  nom_cle     TEXT NOT NULL UNIQUE,
  code        TEXT,
  archivee    INTEGER NOT NULL DEFAULT 0 CHECK (archivee IN (0, 1)),
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
) STRICT;

CREATE TABLE motif (
  id          TEXT PRIMARY KEY,
  nom         TEXT NOT NULL CHECK (length(trim(nom)) > 0),
  nom_cle     TEXT NOT NULL UNIQUE,
  code        TEXT UNIQUE,                            -- 'autre' est réservé (J-04)
  archivee    INTEGER NOT NULL DEFAULT 0 CHECK (archivee IN (0, 1)),
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
) STRICT;

CREATE TABLE semaine_plan (
  lundi         TEXT PRIMARY KEY,                     -- AAAA-MM-JJ, un lundi (3.1)
  type_attendu  TEXT NOT NULL CHECK (type_attendu IN ('entreprise', 'formation', 'aucune')),
  libelle       TEXT CHECK (libelle IS NULL OR length(libelle) <= 100),
  updated_at    TEXT NOT NULL
) STRICT;                                              -- pas de semaine mixte (11/10/2026)

CREATE TABLE fiche (
  id           TEXT PRIMARY KEY,                      -- UUID v7
  lundi        TEXT NOT NULL,
  type_fiche   TEXT NOT NULL CHECK (type_fiche IN ('entreprise', 'formation')),
  statut       TEXT NOT NULL DEFAULT 'brouillon'
               CHECK (statut IN ('brouillon', 'terminee', 'transmise', 'validee')),
  terminee_le  TEXT,                                  -- instant UTC (ponctualité, 3.8)
  transmise_le TEXT,                                  -- date de Paris, seule (F-05)
  validee_le   TEXT,
  modifiee_apres_transmission_le TEXT,                -- instant UTC (F-07)
  version      INTEGER NOT NULL DEFAULT 1,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  UNIQUE (lundi, type_fiche)                          -- R-11
) STRICT;

CREATE TABLE ligne (
  id            TEXT PRIMARY KEY,                     -- UUID v7
  fiche_id      TEXT NOT NULL REFERENCES fiche(id) ON DELETE CASCADE,
  position      INTEGER NOT NULL CHECK (position >= 0),
  date          TEXT NOT NULL,                        -- dans la semaine de la fiche
  mission_id    TEXT NOT NULL REFERENCES mission(id),
  duree_minutes INTEGER NOT NULL CHECK (duree_minutes BETWEEN 15 AND 720 AND duree_minutes % 15 = 0),
  etapes        TEXT NOT NULL DEFAULT '',
  echanges      TEXT,                                 -- échanges avec le tuteur
  UNIQUE (fiche_id, position)
) STRICT;

CREATE TABLE ligne_competence (
  ligne_id       TEXT NOT NULL REFERENCES ligne(id) ON DELETE CASCADE,
  competence_id  TEXT NOT NULL REFERENCES competence(id),
  rang           INTEGER NOT NULL,                    -- ordre de saisie
  PRIMARY KEY (ligne_id, competence_id)
) STRICT;

CREATE TABLE justification (
  id           TEXT PRIMARY KEY,                      -- UUID v7
  lundi        TEXT NOT NULL,
  type_fiche   TEXT NOT NULL CHECK (type_fiche IN ('entreprise', 'formation')),
  motif_id     TEXT NOT NULL REFERENCES motif(id),
  commentaire  TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 1000),
  creee_le     TEXT NOT NULL,                         -- UTC
  levee_le     TEXT                                   -- UTC ; jamais supprimée (J-06)
) STRICT;
CREATE UNIQUE INDEX justification_active ON justification (lundi, type_fiche) WHERE levee_le IS NULL;

CREATE TABLE evenement (                              -- journal en ajout seul (3.9)
  id           TEXT PRIMARY KEY,                      -- UUID v7
  horodatage   TEXT NOT NULL,                         -- UTC ; affiché en Europe/Paris
  type         TEXT NOT NULL CHECK (type IN (
                 'fiche_creee', 'statut_modifie', 'fiche_modifiee_apres_transmission',
                 'fiche_supprimee', 'semaine_justifiee', 'justification_levee',
                 'calendrier_modifie', 'referentiel_modifie', 'import', 'export')),
  cible_type   TEXT NOT NULL,
  cible_id     TEXT NOT NULL,                         -- sans clé étrangère (V-06)
  lundi        TEXT,
  resume       TEXT NOT NULL DEFAULT '',
  avant        TEXT CHECK (avant IS NULL OR json_valid(avant)),
  apres        TEXT CHECK (apres IS NULL OR json_valid(apres))
) STRICT;
CREATE INDEX evenement_recent ON evenement (horodatage DESC, id DESC);
CREATE INDEX evenement_cible ON evenement (cible_type, cible_id);
CREATE TRIGGER evenement_pas_de_update BEFORE UPDATE ON evenement
  BEGIN SELECT RAISE(ABORT, 'journal en ajout seul'); END;
CREATE TRIGGER evenement_pas_de_delete BEFORE DELETE ON evenement
  BEGIN SELECT RAISE(ABORT, 'journal en ajout seul'); END;

CREATE INDEX fiche_par_lundi ON fiche (lundi DESC, id DESC);
CREATE INDEX ligne_par_mission ON ligne (mission_id);
