//! Reusable price-parsing helpers shared across spiders.
//!
//! Spiders extract model IDs cheaply, but pricing lives in heterogeneous
//! tables that each provider formats differently. These helpers turn the two
//! most common patterns — `$X.XX/<unit>` (audio/char pricing) and
//! `$X.XX` qualified by a per-1M-token unit — into numbers the diff engine can
//! compare against the catalog, so price drift gets flagged automatically.
//!
//! All helpers operate on plain text (HTML with tags stripped, or a table-row
//! fragment) and ignore values outside a sanity `range`, so stray dollar
//! amounts (plan prices, credits, etc.) don't get mistaken for token rates.

use std::ops::RangeInclusive;

/// Per-1M-token unit spellings seen across provider pricing pages, lowercased.
/// Matched against the remainder after the amount, with leading whitespace
/// already trimmed off (so no entry should start with a space).
const PER_MILLION_TOKEN_UNITS: &[&str] = &[
    "/1m",
    "/ 1m",
    "/mtok",
    "/m tok",
    "/m token",
    "/1m token",
    "/1m tokens",
    "per 1m",
    "per million",
    "per 1,000,000",
    "/1,000,000",
];

/// Find the first `$<number><unit>` occurrence in `s` whose value falls within
/// `range`, returning the number. `unit` is matched immediately after the
/// number, with any whitespace between the number and unit tolerated
/// (e.g. `"$0.0043/min"` or `"$0.0043 /min"` with `unit = "/min"`).
///
/// Returns `None` if no qualifying price is found.
pub fn first_unit_price(s: &str, unit: &str, range: &RangeInclusive<f64>) -> Option<f64> {
    let unit_lower = unit.to_ascii_lowercase();
    let mut search = s;
    loop {
        let dollar = search.find('$')?;
        let after = &search[dollar + 1..];
        let (num_str, rest) = split_amount(after);
        let rest_lower = rest.trim_start().to_ascii_lowercase();
        if rest_lower.starts_with(&unit_lower)
            && let Some(v) = parse_in_range(&num_str, range)
        {
            return Some(v);
        }
        // Advance past this dollar sign and keep searching.
        search = &search[dollar + 1..];
    }
}

/// Find the first `$<number>` qualified by a per-1M-token unit (e.g. `/1M`,
/// `/MTok`, `per 1M tokens`) whose value falls within `range`.
pub fn first_per_million_token_price(s: &str, range: &RangeInclusive<f64>) -> Option<f64> {
    let mut search = s;
    loop {
        let dollar = search.find('$')?;
        let after = &search[dollar + 1..];
        let (num_str, rest) = split_amount(after);
        let rest_lower = rest.trim_start().to_ascii_lowercase();
        let is_token_price = PER_MILLION_TOKEN_UNITS
            .iter()
            .any(|u| rest_lower.starts_with(u));
        if is_token_price && let Some(v) = parse_in_range(&num_str, range) {
            return Some(v);
        }
        search = &search[dollar + 1..];
    }
}

/// Split a string beginning right after a `$` into (numeric amount, remainder).
/// The amount is the leading run of digits, dots and commas (commas stripped).
fn split_amount(after: &str) -> (String, &str) {
    let end = after
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
        .unwrap_or(after.len());
    let num = after[..end].replace(',', "");
    (num, &after[end..])
}

fn parse_in_range(num_str: &str, range: &RangeInclusive<f64>) -> Option<f64> {
    if num_str.is_empty() {
        return None;
    }
    match num_str.parse::<f64>() {
        Ok(v) if range.contains(&v) => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PER_MIN: RangeInclusive<f64> = 0.001..=0.5;
    const PER_MTOK: RangeInclusive<f64> = 0.0..=10000.0;

    #[test]
    fn unit_price_reads_first_per_minute_rate() {
        let s = "Streaming $0.0077/min. Batch $0.0043/min.";
        assert_eq!(first_unit_price(s, "/min", &PER_MIN), Some(0.0077));
    }

    #[test]
    fn unit_price_tolerates_space_before_unit() {
        assert_eq!(
            first_unit_price("$0.0065 /min", "/min", &PER_MIN),
            Some(0.0065)
        );
    }

    #[test]
    fn unit_price_skips_out_of_range_dollar_amounts() {
        // A $99/mo plan price must not be mistaken for a per-minute rate.
        let s = "Pay $99/mo or $0.0058/min as you go";
        assert_eq!(first_unit_price(s, "/min", &PER_MIN), Some(0.0058));
    }

    #[test]
    fn unit_price_returns_none_when_absent() {
        assert_eq!(first_unit_price("no prices here", "/min", &PER_MIN), None);
    }

    #[test]
    fn per_million_token_price_matches_common_spellings() {
        assert_eq!(
            first_per_million_token_price("$0.50/1M", &PER_MTOK),
            Some(0.50)
        );
        assert_eq!(
            first_per_million_token_price("$3.00 /MTok", &PER_MTOK),
            Some(3.00)
        );
        assert_eq!(
            first_per_million_token_price("Input: $1.25 per 1M tokens", &PER_MTOK),
            Some(1.25)
        );
        assert_eq!(
            first_per_million_token_price("$2,000 per million", &PER_MTOK),
            Some(2000.0)
        );
    }

    #[test]
    fn per_million_token_price_ignores_bare_dollar_amounts() {
        // "$20 credit" has no per-token unit and must be ignored.
        assert_eq!(
            first_per_million_token_price("Get $20 credit. Output $10.00/1M tokens.", &PER_MTOK),
            Some(10.00)
        );
    }
}
