//! India payroll statutory **stub** for automated pay run (v2).
//! Not legal/tax advice: EPF/ESI use simplified percentages; professional tax is a fixed stub above a
//! threshold; TDS is taken from `tax_computation.tds_per_month` when present for the **India FY** of
//! the pay month.

use rust_decimal::Decimal;

/// EPF: employee 12% and employer 12% on the same **PF wage** = `min(basic, ₹15,000)`.
pub const PF_WAGE_CEILING_INR: &str = "15000.00";
/// ESI applies when `gross` is at or below the statutory ceiling (simplified; excludes certain components in practice).
const ESI_GROSS_CEILING: &str = "21000.00";
/// Employee ESI 0.75%, employer 3.25% on gross.
const ESI_EMP_PCT: &str = "0.0075";
const ESI_EMPR_PCT: &str = "0.0325";
/// **Stub** professional tax: ₹200 when `gross > ₹10,000` (state-specific rules are not modelled here).
const PT_GROSS_FLOOR: &str = "10000.00";
const PT_STUB_INR: &str = "200.00";

/// Resultant employer + employee **employee-side** columns for the payslip row.
pub struct IndiaStatutoryStub {
    pub pf_employee: Decimal,
    pub pf_employer: Decimal,
    pub esi_employee: Decimal,
    pub esi_employer: Decimal,
    pub professional_tax: Decimal,
}

/// India financial year “start year” (April–March): e.g. May 2025 → `2025`; Mar 2025 → `2024`.
pub fn india_fy_start_year(cycle_month: i32, cycle_year: i32) -> i32 {
    if (4..=12).contains(&cycle_month) {
        cycle_year
    } else {
        cycle_year - 1
    }
}

fn dec(s: &str) -> Decimal {
    use std::str::FromStr;
    Decimal::from_str(s).expect("const decimal")
}

/// `tds` is the monthly TDS to withhold (e.g. from `tax_computation.tds_per_month`); if none, 0.
pub fn compute(gross: Decimal, tds: Option<Decimal>) -> (IndiaStatutoryStub, Decimal) {
    let tds_m = tds
        .unwrap_or(Decimal::ZERO)
        .round_dp(2);
    let ceiling_pf = dec(PF_WAGE_CEILING_INR);
    let pf_wage = gross.min(ceiling_pf);
    let r12 = dec("0.12");
    let pf_employee = (pf_wage * r12).round_dp(2);
    let pf_employer = (pf_wage * r12).round_dp(2);

    let ceiling_esi = dec(ESI_GROSS_CEILING);
    let (esi_employee, esi_employer) = if gross <= ceiling_esi {
        let a = (gross * dec(ESI_EMP_PCT)).round_dp(2);
        let b = (gross * dec(ESI_EMPR_PCT)).round_dp(2);
        (a, b)
    } else {
        (Decimal::ZERO, Decimal::ZERO)
    };

    let floor_pt = dec(PT_GROSS_FLOOR);
    let professional_tax = if gross > floor_pt {
        dec(PT_STUB_INR)
    } else {
        Decimal::ZERO
    };

    let stat = IndiaStatutoryStub {
        pf_employee,
        pf_employer,
        esi_employee,
        esi_employer,
        professional_tax,
    };
    (stat, tds_m)
}

/// Sum of all employee deductions that reduce net from gross (employer EPF/ESI are **not** included).
pub fn employee_deduction_total(s: &IndiaStatutoryStub, tds: Decimal) -> Decimal {
    s.pf_employee
        + s.esi_employee
        + s.professional_tax
        + tds
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn epf_caps_at_1800() {
        let g = Decimal::from_str("85000").unwrap();
        let (a, tds) = compute(g, None);
        assert_eq!(a.pf_employee, Decimal::from_str("1800.00").unwrap());
        assert_eq!(tds, Decimal::ZERO);
    }
}
