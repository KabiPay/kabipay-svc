# Starts all federated GraphQL subgraphs (4010-4028). Run from repo root or kabipay-svc.
# Requires: cargo build for each crate (use: cargo build -j 1 -p <crate> per crate if full workspace build OOMs on Windows).
$ErrorActionPreference = 'Stop'
$base = Split-Path -Parent $PSScriptRoot
$runs = @(
    @('kabipay-operator.exe', 'KABIPAY_OPERATOR_PORT', '4010'),
    @('kabipay-tenant.exe', 'KABIPAY_TENANT_PORT', '4011'),
    @('kabipay-billing.exe', 'KABIPAY_BILLING_PORT', '4012'),
    @('kabipay-employee.exe', 'KABIPAY_EMPLOYEE_PORT', '4013'),
    @('kabipay-leave.exe', 'KABIPAY_LEAVE_PORT', '4014'),
    @('kabipay-attendance.exe', 'KABIPAY_ATTENDANCE_PORT', '4015'),
    @('kabipay-payroll.exe', 'KABIPAY_PAYROLL_PORT', '4016'),
    @('kabipay-tax.exe', 'KABIPAY_TAX_PORT', '4017'),
    @('kabipay-benefits.exe', 'KABIPAY_BENEFITS_PORT', '4018'),
    @('kabipay-expense.exe', 'KABIPAY_EXPENSE_PORT', '4019'),
    @('kabipay-recruitment.exe', 'KABIPAY_RECRUITMENT_PORT', '4020'),
    @('kabipay-performance.exe', 'KABIPAY_PERFORMANCE_PORT', '4021'),
    @('kabipay-lms.exe', 'KABIPAY_LMS_PORT', '4022'),
    @('kabipay-succession.exe', 'KABIPAY_SUCCESSION_PORT', '4023'),
    @('kabipay-compensation.exe', 'KABIPAY_COMPENSATION_PORT', '4024'),
    @('kabipay-assets.exe', 'KABIPAY_ASSETS_PORT', '4025'),
    @('kabipay-grievance.exe', 'KABIPAY_GRIEVANCE_PORT', '4026'),
    @('kabipay-workflow.exe', 'KABIPAY_WORKFLOW_PORT', '4027'),
    @('kabipay-notification.exe', 'KABIPAY_NOTIFICATION_PORT', '4028')
)
Get-Process -Name 'kabipay-*' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
foreach ($r in $runs) {
    $ex, $k, $v = $r
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = Join-Path $base ("target\debug\" + $ex)
    $psi.WorkingDirectory = $base
    $psi.UseShellExecute = $false
    $psi.EnvironmentVariables[$k] = $v
    [void][System.Diagnostics.Process]::Start($psi)
}
Write-Host "Started $($runs.Count) subgraph processes. GraphQL: http://127.0.0.1:<port>/graphql (4010-4028)."
