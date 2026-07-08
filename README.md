# Steins Reader

A local daemon that tracks your skills on a radar chart and updates
them intelligently using a Kalman filter, instead of just overwriting a
number every time you self-report a score.

```
steins-reader/
├── backend/      Rust (Axum + SQLite) daemon. Owns the Kalman filter, the
│                 SQLite-backed skill/observation store, and the API.
└── frontend/     Vite + React + TypeScript SPA. Renders the radar chart
                  ("the dossier") and the per-skill update panel.
```

## How the filter works (backend/src/kalman.rs)

Each skill is a latent Gaussian `N(mean, variance)`.

1. **Predict** — every time you touch a skill, elapsed time since the last
   update widens `variance` (you get rustier / less certain the longer it's
   been) and gently regresses `mean` toward 50 (`backend/src/kalman.rs`,
   `predict`).
2. **Update** — a new observation comes in with a `measurement_noise` (R)
   determined by its source:
   - `standardized_test` → R = 20 (trusted, moves the estimate hard)
   - `peer_review` → R = 80
   - `self_report` → R = 200
   - `casual` → R = 400 (barely moves a confident estimate)

   Kalman gain `K = variance / (variance + R)` blends the prior mean with
   the new score: `new_mean = mean + K * (score - mean)`, and shrinks
   `variance` accordingly. New skills start with high variance (400) so
   they adapt fast; skills you've fed lots of data into become "sticky" and
   resist single outliers.

Unit tests for this logic live at the bottom of `kalman.rs` — run them with
`cargo test`.

## Running it

### Backend

```powershell
cd backend
cargo run
```

This starts the Axum server on `http://127.0.0.1:1337`, creates
`steins_reader.db` (SQLite, WAL mode) next to the binary on first run, and
seeds the six vertices from the dossier reference image (Intelligence,
Creativity, Willingness to Act, Motivation, Social Life, Acting Skills) at a
neutral 50/high-uncertainty starting point. Edit/delete/add vertices freely
from the UI — there's nothing hardcoded after first boot.

Env vars (all optional):
- `STEINS_CONFIG_PATH` — path to a JSON config file with `port`, `db_path`,
  `frontend_dir`, and `log_dir`
- `STEINS_PORT` — default `1337`
- `STEINS_DB_PATH` — default `steins_reader.db`
- `STEINS_FRONTEND_DIR` — default `dist` in `backend/` after you copy the
  built SPA there, or any other absolute path you want the service to use
- `STEINS_LOG_DIR` — default `logs`

### Frontend (dev mode, hot reload)

```powershell
cd frontend
npm install
npm run dev
```

Vite proxies `/api` to `127.0.0.1:1337` (see `vite.config.ts`), so run the
backend alongside it.

### Production build and service install

Use the platform setup script from the repo root:

```powershell
# Windows (as Administrator)

.\setup.ps1
```

```bash
# Linux
sudo ./setup.sh
```

Both scripts build the frontend, copy the compiled `dist/` folder into the
backend install, build the release binary, stage the files into the platform
install location, write an absolute-path config file, register the service,
and start it on boot.

After install, the Axum server is the only thing you need running. It serves
the API under `/api/*` and the compiled SPA for everything else.

## API

| Method | Path                     | Purpose                                   |
|--------|--------------------------|--------------------------------------------|
| GET    | `/api/skills`            | List all vertices                          |
| POST   | `/api/skills`            | Create a vertex `{ name, initial_mean? }`  |
| PATCH  | `/api/skills/:id`        | Rename / manually override mean/variance   |
| DELETE | `/api/skills/:id`        | Remove a vertex                            |
| POST   | `/api/skills/:id/observe`| Feed a new score through the Kalman filter `{ score, source, note?, measurement_noise? }` |
| GET    | `/api/skills/:id/history`| Full observation log for a vertex          |

## Notes / next steps

- `measurement_noise` per source is currently a constant in
  `models.rs::Source::default_noise` — tune those numbers to taste, or pass
  an explicit `measurement_noise` override per request if you want
  per-question difficulty weighting later.
- The confidence band on the radar chart (the faint gold halo around the
  polygon) visualizes `mean ± 1 stddev` directly from `variance`, so it
  shrinks visibly as a skill accumulates trustworthy data.
- `process_noise` (drift rate) is per-skill and stored in the DB, so you
  could expose a slider for "how fast does this atrophy" per vertex later.
