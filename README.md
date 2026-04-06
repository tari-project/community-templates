# Ootle Community Templates

A community-facing website for discovering, searching, and browsing published Ootle templates.

Templates are synced from a configured Ootle indexer into a PostgreSQL database. Anyone can submit verified off-chain
metadata (description, tags, logo, links) for templates they authored -- the metadata hash is verified against what's
stored on-chain.

## Stack

- **Backend:** Rust (axum, clap, tokio, sqlx)
- **Frontend:** React + TypeScript (Vite)
- **Database:** PostgreSQL with pg_trgm for fuzzy search
- **Auth:** JWT (admin only)

## Quick Start

```bash
# Start PostgreSQL
docker compose up -d db

# Configure
cp config.example.toml config.toml
# Edit config.toml with your indexer URL and JWT secret

# Run the server
cargo run -p ootle-community-templates -- --config config.toml

# In another terminal, start the frontend dev server
cd web && npm install && npm run dev
```

## Docker

```bash
docker compose up
```

This starts both PostgreSQL and the server. The server serves the built frontend as static files.

## Configuration

Configuration is via a TOML file (see `config.example.toml`) with CLI overrides:

```
--port              Override the server port
--database-url      Override the database URL
--indexer-url       Override the indexer URL
--config            Path to config file (default: config.toml)
```

## API

### Public Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/templates/featured` | List featured templates |
| GET | `/api/templates/{addr}` | Get template detail with definition and metadata |
| GET | `/api/search?q=&tags=&category=` | Search/filter templates |
| POST | `/api/templates/{addr}/metadata` | Submit CBOR-encoded metadata |

### Admin Endpoints (JWT required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/auth/login` | Login, returns JWT |
| GET | `/api/admin/templates` | List all templates |
| PUT | `/api/admin/templates/{addr}/featured` | Set featured status |
| PUT | `/api/admin/templates/{addr}/blacklist` | Set blacklist status |
| GET | `/api/admin/admins` | List admins |
| POST | `/api/admin/admins` | Create admin |
| DELETE | `/api/admin/admins/{id}` | Delete admin |
| PUT | `/api/admin/admins/{id}/password` | Change password |

## Metadata Submission

Anyone can submit template metadata as CBOR. The server verifies the hash matches what was published on-chain:

```bash
curl -X POST http://localhost:3000/api/templates/{addr}/metadata \
  -H "Content-Type: application/cbor" \
  --data-binary @metadata.cbor
```

## License

BSD-3-Clause
