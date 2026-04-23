# Run Liquibase tenant changelog **update** against an already-provisioned schema.
# Use this after new changeSets are added (e.g. `timesheet_entry`) so existing tenants get the new tables.
#
# Requires: Docker, postgres on kabipay_default, kabipay-database on the same network as provision-tenant.ps1
#
# Example (demo tenant from seed-demo-data / STATUS):
#   Set-Location D:\work\KabiPay
#   powershell -ExecutionPolicy Bypass -File .\kabipay-svc\scripts\update-tenant-liquibase.ps1 -Schema tenant_342205fc

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^tenant_[a-z0-9_]{1,50}$')]
    [string]$Schema,
    [string]$DbName = 'kabipay_dev',
    [string]$DbUser = 'kabipay',
    [string]$DbPassword = 'changeme',
    [string]$Network = 'kabipay_default'
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$DatabaseDir = Join-Path $RepoRoot 'kabipay-database'
if (-not (Test-Path $DatabaseDir)) {
    throw "kabipay-database not found: $DatabaseDir"
}

$TrackingTable = "${Schema}_databasechangelog"
$TenantPropsPath = Join-Path $DatabaseDir ".generated-tenant-update-$Schema.properties"
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
    Write-Host "==> Liquibase update for schema $Schema (tracking: $Schema.$TrackingTable)..." -ForegroundColor Cyan
    $TenantPropsRel = [System.IO.Path]::GetFileName($TenantPropsPath)
    docker run --rm --network $Network -v "${DatabaseDir}:/liquibase/changelog" -w /liquibase/changelog liquibase/liquibase:4.27 `
        --defaults-file=$TenantPropsRel update
    if ($LASTEXITCODE -ne 0) { throw "Liquibase update failed (exit $LASTEXITCODE)" }
} finally {
    Remove-Item -Path $TenantPropsPath -ErrorAction SilentlyContinue
}
Write-Host "Done. Pending tenant changeSets (if any) are now applied to $Schema." -ForegroundColor Green
