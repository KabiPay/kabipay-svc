<#
.SYNOPSIS
    Provision a KabiPay tenant: ops rows + tenant schema + Liquibase tenant migrations.

.DESCRIPTION
    1. Inserts kabipay_ops.tenant (id = deterministic v5 UUID derived from -Code).
    2. Inserts kabipay_ops.tenant_database referencing the target schema.
    3. CREATE SCHEMA IF NOT EXISTS <Schema> in kabipay_dev.
    4. Runs Liquibase tenant.changelog-master.xml against <Schema>, with a
       per-tenant DATABASECHANGELOG tracking table inside that schema.

    Requires:
      - Docker Desktop running; Postgres up via `kabipay-database/docker-compose.yml`
        (`cd kabipay-database; docker compose up -d postgres`).
      - Compose file sets `name: kabipay`, so the default network is `kabipay_default`.
      - psql available inside the postgres container (it is, for postgres:16-alpine).

.PARAMETER Name
    Human-readable tenant name, e.g. "Demo Co".

.PARAMETER Code
    Short unique code (a-z0-9, 2-32 chars). Also used to seed the deterministic UUID.

.PARAMETER Schema
    OPTIONAL — PostgreSQL schema name to create for this tenant. When omitted, the schema
    is auto-derived as `tenant_<first 8 hex chars of the tenant UUID>`, matching
    `kabipay_common::db::derive_tenant_schema_name`. Pass an explicit value ONLY if you
    have already wired a `kabipay_ops.tenant_database` lookup on the service side.

.PARAMETER Country
    ISO 3166-1 alpha-2 country code. Defaults to IN.

.PARAMETER Currency
    ISO 4217 currency code. Defaults to INR.

.PARAMETER DbName
    Postgres database name. Defaults to kabipay_dev.

.PARAMETER DbUser
    Postgres superuser (used for schema creation + Liquibase). Defaults to kabipay.

.PARAMETER DbPassword
    Password for DbUser. Defaults to changeme.

.PARAMETER PostgresContainer
    Name of the running postgres container. Defaults to kabipay_postgres.

.PARAMETER Network
    Docker network shared with the postgres container. Defaults to kabipay_default.

.EXAMPLE
    .\provision-tenant.ps1 -Name "Demo Co" -Code demo -Schema tenant_demo0001
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-z0-9][a-z0-9_-]{1,31}$')][string]$Code,
    [Parameter(Mandatory = $false)][ValidatePattern('^tenant_[a-z0-9_]{1,50}$')][string]$Schema,
    [string]$Country = 'IN',
    [string]$Currency = 'INR',
    [string]$DbName = 'kabipay_dev',
    [string]$DbUser = 'kabipay',
    [string]$DbPassword = 'changeme',
    [string]$PostgresContainer = 'kabipay_postgres',
    [string]$Network = 'kabipay_default'
)

$ErrorActionPreference = 'Stop'

# Repo root = one level above this script's folder.
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$DatabaseDir = Join-Path $RepoRoot 'kabipay-database'

if (-not (Test-Path $DatabaseDir)) {
    throw "kabipay-database dir not found at $DatabaseDir"
}

function New-DeterministicUuid {
    param([Parameter(Mandatory=$true)][string]$Seed)
    # SHA1(seed) -> first 16 bytes -> set UUID v5 bits -> format.
    $sha1 = [System.Security.Cryptography.SHA1]::Create()
    try {
        $bytes = $sha1.ComputeHash([System.Text.Encoding]::UTF8.GetBytes("kabipay-tenant:$Seed"))[0..15]
        $bytes[6] = ($bytes[6] -band 0x0F) -bor 0x50   # version 5
        $bytes[8] = ($bytes[8] -band 0x3F) -bor 0x80   # variant RFC 4122
        $hex = ($bytes | ForEach-Object { $_.ToString('x2') }) -join ''
        return "$($hex.Substring(0,8))-$($hex.Substring(8,4))-$($hex.Substring(12,4))-$($hex.Substring(16,4))-$($hex.Substring(20,12))"
    } finally { $sha1.Dispose() }
}

$TenantId = New-DeterministicUuid -Seed $Code
$TenantDbRowId = New-DeterministicUuid -Seed "$Code-db"
$Subdomain = $Code.ToLowerInvariant()

# Auto-derive schema to match kabipay_common::db::derive_tenant_schema_name unless the caller
# passed one explicitly. The common resolver uses the first 8 hex chars of the tenant UUID.
if ([string]::IsNullOrWhiteSpace($Schema)) {
    $Schema = 'tenant_' + ($TenantId -replace '-','').Substring(0,8)
    Write-Host "Schema (derived): $Schema"
} else {
    Write-Host "Schema (explicit): $Schema (must match tenant_database.schema_name lookup)"
}

Write-Host "Tenant UUID     : $TenantId"
Write-Host "Tenant DB rowId : $TenantDbRowId"
Write-Host "Subdomain       : $Subdomain"
Write-Host ""

function Invoke-Psql {
    param([Parameter(Mandatory=$true)][string]$Sql)
    $env:PGPASSWORD = $DbPassword
    docker exec -i -e PGPASSWORD=$DbPassword $PostgresContainer `
        psql -U $DbUser -d $DbName -v ON_ERROR_STOP=1 -X -q -c $Sql
    if ($LASTEXITCODE -ne 0) { throw "psql failed (exit $LASTEXITCODE) running: $Sql" }
}

Write-Host "==> Creating tenant schema $Schema ..." -ForegroundColor Cyan
Invoke-Psql "CREATE SCHEMA IF NOT EXISTS $Schema AUTHORIZATION $DbUser;"

Write-Host "==> Upserting kabipay_ops.tenant ..." -ForegroundColor Cyan
$NameEscaped = $Name.Replace("'", "''")
Invoke-Psql @"
INSERT INTO kabipay_ops.tenant (id, name, status, country, currency, subdomain)
VALUES ('$TenantId', '$NameEscaped', 'ACTIVE', '$Country', '$Currency', '$Subdomain')
ON CONFLICT (id) DO UPDATE SET
    name       = EXCLUDED.name,
    status     = EXCLUDED.status,
    country    = EXCLUDED.country,
    currency   = EXCLUDED.currency,
    subdomain  = EXCLUDED.subdomain,
    updated_at = NOW();
"@

Write-Host "==> Upserting kabipay_ops.tenant_database ..." -ForegroundColor Cyan
Invoke-Psql @"
INSERT INTO kabipay_ops.tenant_database (id, tenant_id, db_type, db_host, db_name, schema_name, is_active)
VALUES ('$TenantDbRowId', '$TenantId', 'POSTGRES', 'postgres', '$DbName', '$Schema', true)
ON CONFLICT (id) DO UPDATE SET
    tenant_id   = EXCLUDED.tenant_id,
    db_host     = EXCLUDED.db_host,
    db_name     = EXCLUDED.db_name,
    schema_name = EXCLUDED.schema_name,
    is_active   = true,
    updated_at  = NOW();
"@

Write-Host "==> Running Liquibase tenant changelog against $Schema ..." -ForegroundColor Cyan
$TrackingTable = "${Schema}_databasechangelog"

# Liquibase 4.27 CLI rejects ad-hoc `-D<name>=<value>` arguments. Write a per-tenant
# properties file that substitutes `${schema}` in the changelog via `parameter.schema=...`,
# pins the per-tenant tracking table, and sets the default schema.
$TenantPropsPath = Join-Path $DatabaseDir ".generated-tenant-$Schema.properties"
$TenantProps = @"
changeLogFile=changelog/tenant.changelog-master.xml
url=jdbc:postgresql://postgres:5432/$DbName
username=$DbUser
password=$DbPassword
driver=org.postgresql.Driver
logLevel=INFO
defaultSchemaName=$Schema
databaseChangeLogTableName=$TrackingTable
parameter.schema=$Schema
liquibase.hub.mode=off
"@
$TenantProps | Set-Content -Path $TenantPropsPath -Encoding ASCII

try {
    $TenantPropsRel = [System.IO.Path]::GetFileName($TenantPropsPath)
    $LiquibaseArgs = @(
        '--rm',
        '--network', $Network,
        '-v', "$DatabaseDir`:/liquibase/changelog",
        '-w', '/liquibase/changelog',
        'liquibase/liquibase:4.27',
        "--defaults-file=$TenantPropsRel",
        'update'
    )
    docker run @LiquibaseArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Liquibase tenant migration failed (exit $LASTEXITCODE)."
    }
} finally {
    Remove-Item -Path $TenantPropsPath -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Tenant '$Name' provisioned." -ForegroundColor Green
Write-Host "  id          : $TenantId"
Write-Host "  schema      : $Schema"
Write-Host "  tracking tbl: $Schema.$TrackingTable"
Write-Host ""
Write-Host "Next: run scripts\seed-demo-data.ps1 -TenantId $TenantId -Schema $Schema"
