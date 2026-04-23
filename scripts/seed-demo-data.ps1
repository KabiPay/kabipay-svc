<#
.SYNOPSIS
    Seed deterministic demo rows into a provisioned tenant schema and the
    kabipay_ops (operator) schema so every KabiPay subgraph has something
    to return for its list_* queries.

.DESCRIPTION
    Run this AFTER provision-tenant.ps1. The script is idempotent — every
    INSERT uses ON CONFLICT DO NOTHING, so re-running simply keeps the
    existing rows.

    Seeded coverage:

      Tenant plane ("$Schema"):
        0000 foundation   : department, designation, user, employee (original demo)
        0010 shift/attend : shift (DAY/NIGHT), attendance for today
        0011 leave        : leave_type (CL/SL), leave_request (PENDING)
        0012 payroll      : salary_component (BASIC/HRA), payroll_cycle (current month)
        0013 tax          : tax_configuration_version, tax_slab x 2
        0014 benefits     : benefit_type, benefit_plan
        0015 expense      : expense_category, expense (SUBMITTED)
        0016 recruitment  : job_posting (OPEN), application (APPLIED)
        0018 performance  : review_cycle (ACTIVE), goal (IN_PROGRESS)
        0019 lms          : skill, course
        0020 succession   : competency, talent_pool
        0021 compensation : salary_band, compensation_review_cycle
        0022 assets       : asset_category, asset
        0023 grievance    : grievance_category, grievance_case
        0025 workflow     : workflow, workflow_instance
        0027 comm/audit   : announcement, notification

      Ops plane (kabipay_ops):
        module           : 4 starter modules (EMPLOYEE, LEAVE, PAYROLL, RECRUIT)
        tenant_subscription : 2 active subscriptions for this tenant
        billing_cycle    : current-month cycle
        invoice          : 1 PENDING invoice for the above cycle
        payment          : 1 SUCCEEDED payment on that invoice
        operator_role    : 2 base roles (ADMIN, SUPPORT)
        operator_user    : 1 active admin operator

    Prints the seeded employee UUID at the end — use it as `-e EMPLOYEE_ID`
    when you query the subgraph directly.

.PARAMETER TenantId
    UUID of the tenant registered in kabipay_ops.tenant (produced by provision-tenant.ps1).

.PARAMETER Schema
    Target tenant schema, e.g. tenant_demo0001.

.PARAMETER DbName
    Postgres database name. Defaults to kabipay_dev.

.PARAMETER DbUser
    Postgres user. Defaults to kabipay.

.PARAMETER DbPassword
    Password for DbUser. Defaults to changeme.

.PARAMETER PostgresContainer
    Running postgres container name. Defaults to kabipay_postgres.

.EXAMPLE
    .\seed-demo-data.ps1 -TenantId 5a3b... -Schema tenant_demo0001
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$TenantId,
    [Parameter(Mandatory = $true)][ValidatePattern('^tenant_[a-z0-9_]{1,50}$')][string]$Schema,
    [string]$DbName = 'kabipay_dev',
    [string]$DbUser = 'kabipay',
    [string]$DbPassword = 'changeme',
    [string]$PostgresContainer = 'kabipay_postgres'
)

$ErrorActionPreference = 'Stop'

function New-DeterministicUuid {
    param([Parameter(Mandatory=$true)][string]$Seed)
    $sha1 = [System.Security.Cryptography.SHA1]::Create()
    try {
        $bytes = $sha1.ComputeHash([System.Text.Encoding]::UTF8.GetBytes("kabipay-seed:$Seed"))[0..15]
        $bytes[6] = ($bytes[6] -band 0x0F) -bor 0x50
        $bytes[8] = ($bytes[8] -band 0x3F) -bor 0x80
        $hex = ($bytes | ForEach-Object { $_.ToString('x2') }) -join ''
        return "$($hex.Substring(0,8))-$($hex.Substring(8,4))-$($hex.Substring(12,4))-$($hex.Substring(16,4))-$($hex.Substring(20,12))"
    } finally { $sha1.Dispose() }
}

# ---------- Deterministic UUIDs ----------
# Foundation
$DepartmentId        = New-DeterministicUuid -Seed "${Schema}:dept:engineering"
$DesignationId       = New-DeterministicUuid -Seed "${Schema}:desig:software-engineer"
$UserId              = New-DeterministicUuid -Seed "${Schema}:user:demo"
$EmployeeId          = New-DeterministicUuid -Seed "${Schema}:employee:demo"

# Shift / attendance (0010)
$ShiftDayId          = New-DeterministicUuid -Seed "${Schema}:shift:day"
$ShiftNightId        = New-DeterministicUuid -Seed "${Schema}:shift:night"
$AttendanceTodayId   = New-DeterministicUuid -Seed "${Schema}:attendance:today"

# Leave (0011)
$LeaveTypeClId       = New-DeterministicUuid -Seed "${Schema}:leave_type:cl"
$LeaveTypeSlId       = New-DeterministicUuid -Seed "${Schema}:leave_type:sl"
$LeaveRequest1Id     = New-DeterministicUuid -Seed "${Schema}:leave_request:1"

# Payroll (0012)
$SalaryCompBasicId   = New-DeterministicUuid -Seed "${Schema}:salary_component:basic"
$SalaryCompHraId     = New-DeterministicUuid -Seed "${Schema}:salary_component:hra"
$PayrollCycleId      = New-DeterministicUuid -Seed "${Schema}:payroll_cycle:current"

# Tax (0013)
$TaxConfigId         = New-DeterministicUuid -Seed "${Schema}:tax_config:fy2026"
$TaxSlab1Id          = New-DeterministicUuid -Seed "${Schema}:tax_slab:1"
$TaxSlab2Id          = New-DeterministicUuid -Seed "${Schema}:tax_slab:2"

# Benefits (0014)
$BenefitTypeHealthId = New-DeterministicUuid -Seed "${Schema}:benefit_type:health"
$BenefitPlanId       = New-DeterministicUuid -Seed "${Schema}:benefit_plan:base"

# Expense (0015)
$ExpenseCategoryId   = New-DeterministicUuid -Seed "${Schema}:expense_category:travel"
$ExpenseId           = New-DeterministicUuid -Seed "${Schema}:expense:1"

# Recruitment (0016)
$JobPostingId        = New-DeterministicUuid -Seed "${Schema}:job_posting:swe"
$ApplicationId       = New-DeterministicUuid -Seed "${Schema}:application:1"

# Performance (0018)
$ReviewCycleId       = New-DeterministicUuid -Seed "${Schema}:review_cycle:fy2026h1"
$GoalId              = New-DeterministicUuid -Seed "${Schema}:goal:1"

# LMS (0019)
$SkillId             = New-DeterministicUuid -Seed "${Schema}:skill:rust"
$CourseId            = New-DeterministicUuid -Seed "${Schema}:course:rust-basics"

# Succession (0020)
$CompetencyId        = New-DeterministicUuid -Seed "${Schema}:competency:leadership"
$TalentPoolId        = New-DeterministicUuid -Seed "${Schema}:talent_pool:hipo"

# Compensation (0021)
$SalaryBandId        = New-DeterministicUuid -Seed "${Schema}:salary_band:ic2"
$CompRevCycleId      = New-DeterministicUuid -Seed "${Schema}:comp_review_cycle:2026"

# Assets (0022)
$AssetCategoryId     = New-DeterministicUuid -Seed "${Schema}:asset_category:laptop"
$AssetId             = New-DeterministicUuid -Seed "${Schema}:asset:mbp-01"

# Grievance (0023)
$GrievCategoryId     = New-DeterministicUuid -Seed "${Schema}:grievance_category:hr"
$GrievCaseId         = New-DeterministicUuid -Seed "${Schema}:grievance_case:1"

# Workflow (0025)
$WorkflowId          = New-DeterministicUuid -Seed "${Schema}:workflow:leave-approval"
$WorkflowInstanceId  = New-DeterministicUuid -Seed "${Schema}:workflow_instance:1"

# Notification / Communication (0027)
$AnnouncementId      = New-DeterministicUuid -Seed "${Schema}:announcement:1"
$NotificationId      = New-DeterministicUuid -Seed "${Schema}:notification:1"

# ---- Ops plane UUIDs (tenant-independent constants) ----
$ModuleEmployeeId    = New-DeterministicUuid -Seed "ops:module:EMPLOYEE"
$ModuleLeaveId       = New-DeterministicUuid -Seed "ops:module:LEAVE"
$ModulePayrollId     = New-DeterministicUuid -Seed "ops:module:PAYROLL"
$ModuleRecruitId     = New-DeterministicUuid -Seed "ops:module:RECRUIT"
$OpRoleAdminId       = New-DeterministicUuid -Seed "ops:operator_role:ADMIN"
$OpRoleSupportId     = New-DeterministicUuid -Seed "ops:operator_role:SUPPORT"
$OpUserId            = New-DeterministicUuid -Seed "ops:operator_user:admin"

# Tenant-scoped ops UUIDs
$BillingCycleId      = New-DeterministicUuid -Seed "ops:billing_cycle:${TenantId}:current"
$InvoiceId           = New-DeterministicUuid -Seed "ops:invoice:${TenantId}:1"
$PaymentId           = New-DeterministicUuid -Seed "ops:payment:${TenantId}:1"
$SubLeaveId          = New-DeterministicUuid -Seed "ops:subscription:${TenantId}:LEAVE"
$SubPayrollId        = New-DeterministicUuid -Seed "ops:subscription:${TenantId}:PAYROLL"

Write-Host "=== Tenant plane UUIDs ==="
Write-Host "Department   : $DepartmentId"
Write-Host "Designation  : $DesignationId"
Write-Host "User         : $UserId"
Write-Host "Employee     : $EmployeeId"
Write-Host ""

# Argon2id hash for password "ChangeMe!123" (verified — generated by
# `cargo run -p kabipay-auth --bin kabipay-auth-hash --release -- "ChangeMe!123"`).
# Re-run that command if you rotate the demo password.
$PasswordHash = '$argon2id$v=19$m=19456,t=2,p=1$CDQNnKaKe519h5WXXU1DaA$IiZxOr7AvMrrMg0U2q2L1bD5CsBxDVWCHY42+CnLTXw'

function Invoke-TenantSql {
    param([Parameter(Mandatory=$true)][string]$Sql, [string]$Label)
    if ($Label) { Write-Host "==> $Label" -ForegroundColor Cyan }
    docker exec -i -e PGPASSWORD=$DbPassword $PostgresContainer `
        psql -U $DbUser -d $DbName -v ON_ERROR_STOP=1 -X -c $Sql
    if ($LASTEXITCODE -ne 0) {
        throw "Seed step '$Label' failed (exit $LASTEXITCODE)."
    }
}

# =====================================================================
# 1. FOUNDATION (unchanged)
# =====================================================================
$SqlFoundation = @"
INSERT INTO "$Schema".department (id, tenant_id, name, code)
VALUES ('$DepartmentId', '$TenantId', 'Engineering', 'ENG')
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".designation (id, tenant_id, department_id, title, level, grade)
VALUES ('$DesignationId', '$TenantId', '$DepartmentId', 'Software Engineer', 'IC2', 2)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema"."user" (id, tenant_id, email, password_hash, is_active, mfa_enabled)
VALUES ('$UserId', '$TenantId', 'demo@kabipay.local', '$PasswordHash', true, false)
ON CONFLICT (id) DO UPDATE SET password_hash = EXCLUDED.password_hash, is_active = true;

INSERT INTO "$Schema".employee (
    id, tenant_id, user_id, department_id, designation_id,
    employee_code, first_name, last_name, employment_type, status,
    date_of_joining
) VALUES (
    '$EmployeeId', '$TenantId', '$UserId', '$DepartmentId', '$DesignationId',
    'EMP0001', 'Demo', 'Employee', 'PERMANENT', 'ACTIVE',
    CURRENT_DATE
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0000 foundation (department, designation, user, employee)" -Sql $SqlFoundation

# =====================================================================
# 2. SHIFT / ATTENDANCE (0010)
# =====================================================================
$SqlShift = @"
INSERT INTO "$Schema".shift (id, tenant_id, name, start_time, end_time, work_hours, is_night_shift)
VALUES ('$ShiftDayId', '$TenantId', 'General Day', '09:00', '18:00', 9, false)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".shift (id, tenant_id, name, start_time, end_time, work_hours, is_night_shift)
VALUES ('$ShiftNightId', '$TenantId', 'Night Ops', '22:00', '07:00', 9, true)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".attendance (
    id, tenant_id, employee_id, shift_id, work_date,
    check_in_time, check_out_time, status, source
) VALUES (
    '$AttendanceTodayId', '$TenantId', '$EmployeeId', '$ShiftDayId', CURRENT_DATE,
    '09:15', '18:10', 'PRESENT', 'WEB'
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0010 shift + attendance" -Sql $SqlShift

# =====================================================================
# 3. LEAVE (0011)
# =====================================================================
$SqlLeave = @"
INSERT INTO "$Schema".leave_type (
    id, tenant_id, name, code,
    is_paid, carry_forward, max_carry_forward_days,
    sandwich_rule, half_day_allowed, requires_document
) VALUES
    ('$LeaveTypeClId', '$TenantId', 'Casual Leave', 'CL', true,  true,  5, false, true,  false),
    ('$LeaveTypeSlId', '$TenantId', 'Sick Leave',   'SL', true,  false, 0, false, true,  true)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".leave_request (
    id, tenant_id, employee_id, leave_type_id,
    from_date, to_date, days_requested,
    is_half_day, status, reason, applied_at
) VALUES (
    '$LeaveRequest1Id', '$TenantId', '$EmployeeId', '$LeaveTypeClId',
    CURRENT_DATE + INTERVAL '7 days', CURRENT_DATE + INTERVAL '8 days', 2,
    false, 'PENDING', 'Family function', NOW()
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0011 leave (types + request)" -Sql $SqlLeave

# =====================================================================
# 4. PAYROLL (0012)
# =====================================================================
$SqlPayroll = @"
INSERT INTO "$Schema".salary_component (
    id, tenant_id, name, code, type, is_taxable, is_fixed, is_active
) VALUES
    ('$SalaryCompBasicId', '$TenantId', 'Basic',             'BASIC', 'EARNING', true, true, true),
    ('$SalaryCompHraId',   '$TenantId', 'House Rent Allow.', 'HRA',   'EARNING', true, true, true)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".payroll_cycle (
    id, tenant_id, name, month, year, status, payment_date
) VALUES (
    '$PayrollCycleId', '$TenantId',
    TO_CHAR(CURRENT_DATE, 'FMMonth YYYY'),
    EXTRACT(MONTH FROM CURRENT_DATE)::int,
    EXTRACT(YEAR FROM CURRENT_DATE)::int,
    'DRAFT',
    (DATE_TRUNC('month', CURRENT_DATE) + INTERVAL '1 month - 1 day')::date
)
ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0012 payroll (components + cycle)" -Sql $SqlPayroll

# =====================================================================
# 5. TAX (0013)
# =====================================================================
$SqlTax = @"
INSERT INTO "$Schema".tax_configuration_version (
    id, tenant_id, fiscal_year, regime, country_code, is_active
) VALUES (
    '$TaxConfigId', '$TenantId', 2026, 'NEW', 'IN', true
) ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".tax_slab (
    id, tenant_id, tax_config_version_id,
    income_from, income_to, tax_rate, surcharge_rate, cess_rate
) VALUES
    ('$TaxSlab1Id', '$TenantId', '$TaxConfigId',        0,  300000, 0,  0, 4),
    ('$TaxSlab2Id', '$TenantId', '$TaxConfigId',   300000,  600000, 5,  0, 4)
ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0013 tax (configuration + slabs)" -Sql $SqlTax

# =====================================================================
# 6. BENEFITS (0014)
# =====================================================================
$SqlBenefits = @"
INSERT INTO "$Schema".benefit_type (id, tenant_id, name, code, category)
VALUES ('$BenefitTypeHealthId', '$TenantId', 'Group Health Insurance', 'GHI', 'INSURANCE')
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".benefit_plan (
    id, tenant_id, benefit_type_id, name,
    employer_contribution, employee_contribution, contribution_type,
    is_mandatory, is_active
) VALUES (
    '$BenefitPlanId', '$TenantId', '$BenefitTypeHealthId', 'GHI Base Plan',
    8000, 2000, 'FLAT', false, true
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0014 benefits (type + plan)" -Sql $SqlBenefits

# =====================================================================
# 7. EXPENSE (0015)
# =====================================================================
$SqlExpense = @"
INSERT INTO "$Schema".expense_category (id, tenant_id, name, code, max_amount_per_claim)
VALUES ('$ExpenseCategoryId', '$TenantId', 'Travel', 'TRAVEL', 50000)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".expense (
    id, tenant_id, employee_id, expense_category_id,
    amount, currency, expense_date, title, status, submitted_at
) VALUES (
    '$ExpenseId', '$TenantId', '$EmployeeId', '$ExpenseCategoryId',
    4500, 'INR', CURRENT_DATE - INTERVAL '3 days', 'Client Visit - Cab & Meals', 'SUBMITTED', NOW()
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0015 expense (category + submitted expense)" -Sql $SqlExpense

# =====================================================================
# 8. RECRUITMENT (0016)
# =====================================================================
$SqlRecruitment = @"
INSERT INTO "$Schema".job_posting (
    id, tenant_id, department_id, designation_id,
    title, description, employment_type, vacancies, status,
    open_date, close_date
) VALUES (
    '$JobPostingId', '$TenantId', '$DepartmentId', '$DesignationId',
    'Senior Software Engineer', 'Rust + TypeScript, remote friendly.', 'PERMANENT', 2, 'OPEN',
    CURRENT_DATE, CURRENT_DATE + INTERVAL '30 days'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".application (
    id, tenant_id, job_id, candidate_name, candidate_email, candidate_phone,
    source, status, applied_at
) VALUES (
    '$ApplicationId', '$TenantId', '$JobPostingId',
    'Asha Rao', 'asha.rao@example.com', '+91-9999999999',
    'LINKEDIN', 'APPLIED', NOW()
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0016 recruitment (job posting + application)" -Sql $SqlRecruitment

# =====================================================================
# 9. PERFORMANCE (0018)
# =====================================================================
$SqlPerformance = @"
INSERT INTO "$Schema".review_cycle (
    id, tenant_id, name, start_date, end_date, status, review_type
) VALUES (
    '$ReviewCycleId', '$TenantId', 'FY26 H1',
    DATE_TRUNC('year', CURRENT_DATE)::date,
    (DATE_TRUNC('year', CURRENT_DATE) + INTERVAL '6 months - 1 day')::date,
    'ACTIVE', 'HALF_YEARLY'
) ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".goal (
    id, tenant_id, employee_id, review_cycle_id,
    title, description, weightage, status, visibility
) VALUES (
    '$GoalId', '$TenantId', '$EmployeeId', '$ReviewCycleId',
    'Ship federated gateway', 'Unblock module delivery via graphql-yoga stitching.',
    40, 'IN_PROGRESS', 'MANAGER'
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0018 performance (review cycle + goal)" -Sql $SqlPerformance

# =====================================================================
# 10. LMS (0019)
# =====================================================================
$SqlLms = @"
INSERT INTO "$Schema".skill (id, tenant_id, name, category, level)
VALUES ('$SkillId', '$TenantId', 'Rust', 'PROGRAMMING', 'INTERMEDIATE')
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".course (
    id, tenant_id, title, description, category, delivery_mode,
    duration_minutes, is_mandatory, is_active
) VALUES (
    '$CourseId', '$TenantId', 'Rust Basics',
    'Ownership, borrowing, lifetimes and async essentials.',
    'ENGINEERING', 'SELF_PACED',
    240, false, true
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0019 lms (skill + course)" -Sql $SqlLms

# =====================================================================
# 11. SUCCESSION (0020)
# =====================================================================
$SqlSuccession = @"
INSERT INTO "$Schema".competency (id, tenant_id, name, category, description)
VALUES ('$CompetencyId', '$TenantId', 'Leadership', 'CORE', 'Guides teams, sets direction, coaches.')
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".talent_pool (id, tenant_id, name, description)
VALUES ('$TalentPoolId', '$TenantId', 'High Potential 2026', 'Top 10% identified in FY26 H1 review.')
ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0020 succession (competency + talent pool)" -Sql $SqlSuccession

# =====================================================================
# 12. COMPENSATION (0021)
# =====================================================================
$SqlCompensation = @"
INSERT INTO "$Schema".salary_band (
    id, tenant_id, designation_id, grade,
    min_salary, mid_salary, max_salary, currency, effective_year
) VALUES (
    '$SalaryBandId', '$TenantId', '$DesignationId', 2,
    1200000, 1600000, 2000000, 'INR', 2026
) ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".compensation_review_cycle (
    id, tenant_id, name, year, start_date, end_date, status, budget_percentage
) VALUES (
    '$CompRevCycleId', '$TenantId', 'Annual Comp 2026', 2026,
    DATE_TRUNC('year', CURRENT_DATE)::date,
    (DATE_TRUNC('year', CURRENT_DATE) + INTERVAL '1 year - 1 day')::date,
    'PLANNING', 8
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0021 compensation (salary band + review cycle)" -Sql $SqlCompensation

# =====================================================================
# 13. ASSETS (0022)
# =====================================================================
$SqlAssets = @"
INSERT INTO "$Schema".asset_category (id, tenant_id, name, code)
VALUES ('$AssetCategoryId', '$TenantId', 'Laptops', 'LAPTOP')
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".asset (
    id, tenant_id, asset_category_id, name,
    serial_number, asset_tag, purchase_value, purchase_date, status
) VALUES (
    '$AssetId', '$TenantId', '$AssetCategoryId', 'MacBook Pro 14 inch',
    'MBP14-2026-0001', 'KPA-MBP-0001', 210000, CURRENT_DATE - INTERVAL '30 days', 'AVAILABLE'
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0022 assets (category + asset)" -Sql $SqlAssets

# =====================================================================
# 14. GRIEVANCE (0023)
# =====================================================================
$SqlGrievance = @"
INSERT INTO "$Schema".grievance_category (id, tenant_id, name, code, is_posh, resolution_sla_days)
VALUES ('$GrievCategoryId', '$TenantId', 'HR Policy', 'HR_POLICY', false, 14)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".grievance_case (
    id, tenant_id, employee_id, grievance_category_id,
    subject, description, status, priority, confidentiality_level, filed_at
) VALUES (
    '$GrievCaseId', '$TenantId', '$EmployeeId', '$GrievCategoryId',
    'Request for remote work policy clarification',
    'Need clarification on multi-state remote work policy.',
    'OPEN', 'MEDIUM', 'STANDARD', NOW()
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0023 grievance (category + case)" -Sql $SqlGrievance

# =====================================================================
# 15. WORKFLOW (0025)
# =====================================================================
$SqlWorkflow = @"
INSERT INTO "$Schema".workflow (id, tenant_id, name, entity_type, is_active)
VALUES ('$WorkflowId', '$TenantId', 'Leave Approval', 'LEAVE_REQUEST', true)
ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".workflow_instance (
    id, tenant_id, workflow_id, entity_type, entity_id, status
) VALUES (
    '$WorkflowInstanceId', '$TenantId', '$WorkflowId',
    'LEAVE_REQUEST', '$LeaveRequest1Id', 'IN_PROGRESS'
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0025 workflow (workflow + instance)" -Sql $SqlWorkflow

# =====================================================================
# 16. NOTIFICATION / COMMUNICATION (0027)
# =====================================================================
$SqlComm = @"
INSERT INTO "$Schema".announcement (
    id, tenant_id, created_by, title, body, target_audience, publish_at
) VALUES (
    '$AnnouncementId', '$TenantId', '$UserId',
    'Welcome to KabiPay!',
    'Thanks for provisioning your tenant. Explore the modules via /admin/module-health.',
    'ALL', NOW()
) ON CONFLICT (id) DO NOTHING;

INSERT INTO "$Schema".notification (
    id, tenant_id, user_id, type, title, message, action_url, is_read
) VALUES (
    '$NotificationId', '$TenantId', '$UserId',
    'SYSTEM', 'Demo data seeded',
    'Your tenant now has sample rows across all modules.',
    '/admin/module-health', false
) ON CONFLICT (id) DO NOTHING;
"@
Invoke-TenantSql -Label "0027 communication (announcement + notification)" -Sql $SqlComm

# =====================================================================
# 17. OPS PLANE (kabipay_ops)
#     modules, subscriptions, billing_cycle, invoice, payment, operator_*
# =====================================================================
$SqlOps = @"
-- Module catalogue (4 starter modules, idempotent)
INSERT INTO kabipay_ops.module (id, code, name, category, description, is_active, display_order, is_core) VALUES
    ('$ModuleEmployeeId', 'EMPLOYEE',    'Employee Core',       'CORE', 'Master employee records.',        true, 10, true),
    ('$ModuleLeaveId',    'LEAVE',       'Leave Management',    'HR',   'Policies, balances, requests.',   true, 20, false),
    ('$ModulePayrollId',  'PAYROLL',     'Payroll Processing',  'HR',   'Cycles, payslips, compliance.',   true, 30, false),
    ('$ModuleRecruitId',  'RECRUITMENT', 'Talent Acquisition',  'HR',   'Job postings and applications.',  true, 40, false)
ON CONFLICT (id) DO NOTHING;

-- Two active subscriptions for this tenant
INSERT INTO kabipay_ops.tenant_subscription (
    id, tenant_id, module_id, status,
    activated_at, expires_at,
    contracted_seats, current_seat_usage, overage_policy
) VALUES
    ('$SubLeaveId',   '$TenantId', '$ModuleLeaveId',   'ACTIVE',
        CURRENT_DATE, CURRENT_DATE + INTERVAL '1 year', 100, 1, 'BLOCK'),
    ('$SubPayrollId', '$TenantId', '$ModulePayrollId', 'ACTIVE',
        CURRENT_DATE, CURRENT_DATE + INTERVAL '1 year', 100, 1, 'BLOCK')
ON CONFLICT (id) DO NOTHING;

-- Current-month billing cycle
INSERT INTO kabipay_ops.billing_cycle (
    id, tenant_id, period_start, period_end, frequency, status
) VALUES (
    '$BillingCycleId', '$TenantId',
    DATE_TRUNC('month', CURRENT_DATE)::date,
    (DATE_TRUNC('month', CURRENT_DATE) + INTERVAL '1 month - 1 day')::date,
    'MONTHLY', 'INVOICED'
) ON CONFLICT (id) DO NOTHING;

-- One pending invoice for that cycle
INSERT INTO kabipay_ops.invoice (
    id, tenant_id, billing_cycle_id, invoice_number,
    subtotal, discount_total, tax_amount, total_amount, currency,
    status, due_date
) VALUES (
    '$InvoiceId', '$TenantId', '$BillingCycleId',
    CONCAT('INV-', TO_CHAR(CURRENT_DATE, 'YYYYMM'), '-', SUBSTRING('$TenantId', 1, 8)),
    20000, 0, 3600, 23600, 'INR',
    'PENDING', CURRENT_DATE + INTERVAL '15 days'
) ON CONFLICT (id) DO NOTHING;

-- Successful payment for that invoice
INSERT INTO kabipay_ops.payment (
    id, invoice_id, amount, payment_method, status, paid_at, gateway_ref
) VALUES (
    '$PaymentId', '$InvoiceId', 23600, 'CARD', 'SUCCEEDED', NOW(), 'demo_pay_0001'
) ON CONFLICT (id) DO NOTHING;

-- Operator RBAC seed
INSERT INTO kabipay_ops.operator_role (id, code, name, description) VALUES
    ('$OpRoleAdminId',   'ADMIN',   'Platform Admin',   'Full access to all tenants and operator tools.'),
    ('$OpRoleSupportId', 'SUPPORT', 'Support Engineer', 'Read-only tenant inspection for support.')
ON CONFLICT (id) DO NOTHING;

INSERT INTO kabipay_ops.operator_user (
    id, email, password_hash, full_name, phone, is_active
) VALUES (
    '$OpUserId',
    'ops-admin@kabipay.local',
    '$PasswordHash',
    'Ops Admin',
    '+91-9000000001',
    true
) ON CONFLICT (id) DO UPDATE SET password_hash = EXCLUDED.password_hash, is_active = true;
"@
Invoke-TenantSql -Label "ops plane (modules, subscriptions, billing, operators)" -Sql $SqlOps

# =====================================================================
# 18. SUMMARY — counts across seeded tables
# =====================================================================
$SqlSummary = @"
SELECT 'tenant.employee'       AS table, COUNT(*) AS rows FROM "$Schema".employee
UNION ALL SELECT 'tenant.shift',              COUNT(*) FROM "$Schema".shift
UNION ALL SELECT 'tenant.attendance',         COUNT(*) FROM "$Schema".attendance
UNION ALL SELECT 'tenant.leave_type',         COUNT(*) FROM "$Schema".leave_type
UNION ALL SELECT 'tenant.leave_request',      COUNT(*) FROM "$Schema".leave_request
UNION ALL SELECT 'tenant.salary_component',   COUNT(*) FROM "$Schema".salary_component
UNION ALL SELECT 'tenant.payroll_cycle',      COUNT(*) FROM "$Schema".payroll_cycle
UNION ALL SELECT 'tenant.tax_config_ver',     COUNT(*) FROM "$Schema".tax_configuration_version
UNION ALL SELECT 'tenant.tax_slab',           COUNT(*) FROM "$Schema".tax_slab
UNION ALL SELECT 'tenant.benefit_plan',       COUNT(*) FROM "$Schema".benefit_plan
UNION ALL SELECT 'tenant.expense',            COUNT(*) FROM "$Schema".expense
UNION ALL SELECT 'tenant.job_posting',        COUNT(*) FROM "$Schema".job_posting
UNION ALL SELECT 'tenant.application',        COUNT(*) FROM "$Schema".application
UNION ALL SELECT 'tenant.review_cycle',       COUNT(*) FROM "$Schema".review_cycle
UNION ALL SELECT 'tenant.goal',               COUNT(*) FROM "$Schema".goal
UNION ALL SELECT 'tenant.skill',              COUNT(*) FROM "$Schema".skill
UNION ALL SELECT 'tenant.course',             COUNT(*) FROM "$Schema".course
UNION ALL SELECT 'tenant.competency',         COUNT(*) FROM "$Schema".competency
UNION ALL SELECT 'tenant.talent_pool',        COUNT(*) FROM "$Schema".talent_pool
UNION ALL SELECT 'tenant.salary_band',        COUNT(*) FROM "$Schema".salary_band
UNION ALL SELECT 'tenant.comp_review_cycle',  COUNT(*) FROM "$Schema".compensation_review_cycle
UNION ALL SELECT 'tenant.asset',              COUNT(*) FROM "$Schema".asset
UNION ALL SELECT 'tenant.grievance_case',     COUNT(*) FROM "$Schema".grievance_case
UNION ALL SELECT 'tenant.workflow_instance',  COUNT(*) FROM "$Schema".workflow_instance
UNION ALL SELECT 'tenant.announcement',       COUNT(*) FROM "$Schema".announcement
UNION ALL SELECT 'tenant.notification',       COUNT(*) FROM "$Schema".notification
UNION ALL SELECT 'ops.module',                COUNT(*) FROM kabipay_ops.module
UNION ALL SELECT 'ops.tenant_subscription',   COUNT(*) FROM kabipay_ops.tenant_subscription WHERE tenant_id = '$TenantId'
UNION ALL SELECT 'ops.invoice',               COUNT(*) FROM kabipay_ops.invoice WHERE tenant_id = '$TenantId'
UNION ALL SELECT 'ops.payment',               COUNT(*) FROM kabipay_ops.payment p
    JOIN kabipay_ops.invoice i ON i.id = p.invoice_id WHERE i.tenant_id = '$TenantId'
UNION ALL SELECT 'ops.operator_user',         COUNT(*) FROM kabipay_ops.operator_user
ORDER BY 1;
"@
Invoke-TenantSql -Label "counts" -Sql $SqlSummary

Write-Host ""
Write-Host "Seed complete." -ForegroundColor Green
Write-Host ""
Write-Host "Try the employee query once kabipay-employee is running:" -ForegroundColor Yellow
Write-Host '  PowerShell:'
Write-Host ('    Invoke-RestMethod -Method Post -Uri http://127.0.0.1:4013/graphql ' +
            '-Headers @{"content-type"="application/json"; "x-tenant-id"="' + $TenantId + '"} ' +
            '-Body (''{"query":"{ employee(id: \"' + $EmployeeId + '\") { id employeeCode firstName lastName fullName } }"}'')')
Write-Host ''
Write-Host '  curl (single-line):'
$Query = '{ employee(id: \"' + $EmployeeId + '\") { id employeeCode firstName lastName fullName } }'
$JsonBody = '{"query":"' + $Query + '"}'
Write-Host ('    curl -s -X POST http://127.0.0.1:4013/graphql -H "content-type: application/json" ' +
            '-H "x-tenant-id: ' + $TenantId + '" -d ''' + $JsonBody + '''')
Write-Host ''
Write-Host "Or point the UI at /admin/module-health for a green-light matrix across all subgraphs." -ForegroundColor Yellow
