# kabipay-svc

Rust workspace for **kabipay-auth** (REST) and federated **GraphQL subgraphs** (employee, leave, payroll, etc.). Each crate exposes HTTP on its own port; the **kabipay-gateway** service stitches them into one GraphQL endpoint.

## Dependencies

| Requirement | Notes |
|-------------|--------|
| **Rust** | Stable toolchain (`rustup`, `cargo`). |
| **PostgreSQL 16** | Database with **ops** schema (`kabipay_ops`) and per-tenant schemas created via **kabipay-database** (Liquibase). |
| **Environment** | Copy `.env.example` → `.env` in this folder, or export the same variables. Services call `dotenvy` where configured. |

Optional:

- **PowerShell** (Windows) for `scripts/*.ps1` helpers.

## Configure

1. Ensure Postgres is running and migrations are applied (see **kabipay-database** README: ops changelog first, then tenant schema(s)).

2. In this directory:

   ```powershell
   copy .env.example .env
   ```

   Edit `.env`:

   - Set **`DATABASE_URL`** *or* **`POSTGRES_HOST`**, **`POSTGRES_PORT`**, **`POSTGRES_DB`**, **`POSTGRES_USER`**, **`POSTGRES_PASSWORD`** (and **`POSTGRES_SSLMODE`** for managed providers) so they match your database.
   - Set **`KABIPAY_CLIENT_JWT_SECRET`** and **`KABIPAY_OPERATOR_JWT_SECRET`** to long random strings (32+ characters).

## Build

From this directory:

```powershell
cargo build --workspace
```

On memory-constrained Windows machines, build crates one at a time if needed:

```powershell
cargo build -j 1 -p kabipay-auth
```

## Run services

### Auth (REST)

```powershell
cargo run -p kabipay-auth
```

Default port: **`4001`** (`KABIPAY_AUTH_PORT`).

### One subgraph (example)

```powershell
cargo run -p kabipay-employee
```

GraphQL: **`http://127.0.0.1:4013/graphql`** (port from `KABIPAY_EMPLOYEE_PORT` in `.env`).

### All subgraphs (Windows helper)

After a **release** or **debug** build, from this directory:

```powershell
.\scripts\start-subgraphs.ps1
```

The script starts **debug** binaries under `target\debug\kabipay-*.exe` on ports **4010–4028**. Ensure **`kabipay-auth`** is started separately if the UI or gateway needs login.

## Scripts (optional)

| Script | Purpose |
|--------|---------|
| `scripts\provision-tenant.ps1` | Ops rows + tenant schema + Liquibase tenant migrations (Node + `kabipay-database` `npm install`). |
| `scripts\update-tenant-liquibase.ps1` | Re-run tenant migrations for an existing schema. |
| `scripts\seed-demo-data.ps1` | Demo seed data (requires DB + tenant already provisioned). |

Adjust paths inside scripts if **kabipay-database** is not a sibling folder.

## Ports (defaults)

| Service | Env var | Default port |
|---------|---------|--------------|
| Auth REST | `KABIPAY_AUTH_PORT` | 4001 |
| Operator subgraph | `KABIPAY_OPERATOR_PORT` | 4010 |
| … | … | 4011–4028 |

Exact mapping is in `.env.example` and `scripts\start-subgraphs.ps1`.

## Related repositories

- **kabipay-database** — Liquibase schema migrations.
- **kabipay-gateway** — federated GraphQL gateway; point it at the same subgraph base URL and ports.
- **kabipay-ui** — browser client; `public/config.json` must match auth URL and gateway URL.
