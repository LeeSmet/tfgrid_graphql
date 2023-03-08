use std::collections::BTreeMap;

use crate::compat::{de_i64, de_u64};
use chrono::{TimeZone, Utc};
use plotters::prelude::*;
use serde::{Deserialize, Serialize};

/// A contract bill report on the grid.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBillReport {
    #[serde(deserialize_with = "de_u64")]
    pub amount_billed: u64,
    #[serde(rename = "contractID", deserialize_with = "de_u64")]
    pub contract_id: u64,
    #[serde(deserialize_with = "de_i64")]
    pub timestamp: i64,
    pub discount_received: DiscountLevel,
}

#[derive(Serialize, Deserialize)]
/// Level of discount applied for a contract bill.
pub enum DiscountLevel {
    None,
    Default,
    Bronze,
    Silver,
    Gold,
}

/// Create a graph of the amount of bills per hour. The input bill reports must be sorted in order
/// of ascending timestamp.
///
/// # Panics
///
/// This function panics if an empty slice is passed as argument, or the reports are not sorted
/// based on timestamp.
pub fn graph_billed_per_hour(
    reports: &[ContractBillReport],
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    assert!(!reports.is_empty());
    // TODO: panic if not sorted

    let mut hour_payments = BTreeMap::<_, u64>::new();
    let mut reports_in_hour = BTreeMap::<_, usize>::new();
    for report in reports {
        *hour_payments.entry(report.timestamp / 3600).or_default() += report.amount_billed;
        *reports_in_hour.entry(report.timestamp / 3600).or_default() += 1;
    }

    // Since reports is not empty we always have a first and last entry
    let min_key = *hour_payments.first_entry().unwrap().key();
    let max_key = *hour_payments.last_entry().unwrap().key();
    let min_bill = hour_payments.values().min().unwrap();
    let max_bill = hour_payments.values().max().unwrap();
    let min_bill_reports = reports_in_hour.values().min().unwrap();
    let max_bill_reports = reports_in_hour.values().max().unwrap();

    let root_area = SVGBackend::new("./chart.svg", (1920, 1080)).into_drawing_area();
    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .set_label_area_size(LabelAreaPosition::Right, 40)
        .caption("Total contract billed over time", ("sans-serif", 40))
        .build_cartesian_2d(min_key..max_key, min_bill * 8 / 10..max_bill * 12 / 10)?
        .set_secondary_coord(
            min_key..max_key,
            min_bill_reports * 8 / 10..max_bill_reports * 12 / 10,
        );

    let x_formatter =
        Box::new(|&v: &i64| -> String { Utc.timestamp_opt(v * 3600, 0).unwrap().to_string() });
    let y_formatter =
        Box::new(|&v: &u64| -> String { format!("{}.{} TFT", v / 10_000_000, v % 10_000_000) });

    ctx.configure_mesh()
        .x_labels(10)
        .x_label_formatter(&x_formatter)
        .y_labels(10)
        .y_label_formatter(&y_formatter)
        .draw()?;

    ctx.configure_secondary_axes()
        .y_desc("Bill reports in hour")
        .draw()?;

    let hist = Histogram::vertical(&ctx)
        .style(GREEN.filled())
        .margin(5)
        .data(hour_payments);
    ctx.draw_series(hist)?;

    ctx.draw_secondary_series(LineSeries::new(reports_in_hour, &RED))?;

    ctx.configure_series_labels().draw()?;

    Ok(())
}
