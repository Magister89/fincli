//! Pure quote valuation and session P&L policy.
//!
//! Everything in this module is plain data in, plain data out. Timestamps are
//! Unix milliseconds and every function takes an injected `now`, so callers
//! (portfolio enrichment, currency grouping, tests) share one implementation.
//! The policy itself is documented in `docs/quote-policy.md`.

use chrono::{DateTime, NaiveDate, Utc};

/// A timestamp at most this far in the future is tolerated (clock skew).
const MAX_FUTURE_MS: i64 = 5 * 60 * 1000;
/// Maximum age of the provider quote time for session eligibility.
const MAX_QUOTE_AGE_MS: i64 = 96 * 60 * 60 * 1000;
/// Maximum age of the local fetch time for session eligibility.
const MAX_FETCH_AGE_MS: i64 = 120 * 1000;

/// How a price/previous-close comparison must be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonPeriod {
    /// Intraday move between the previous session close and the latest quote.
    Session,
    /// Move between consecutive published valuations (e.g. fund NAVs).
    ValuationPeriod,
    /// The provider gave too little information to classify the comparison.
    Unknown,
}

impl ComparisonPeriod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Session => "session period",
            Self::ValuationPeriod => "valuation period",
            Self::Unknown => "unknown period",
        }
    }
}

/// Comparison period implied by the provider instrument/quote type.
///
/// The configured portfolio allocation is deliberately not consulted: the
/// provider type is the only signal here, and unknown types stay unknown.
pub fn comparison_period(provider_type: Option<&str>) -> ComparisonPeriod {
    match provider_type {
        Some(provider_type) => match provider_type.trim().to_ascii_uppercase().as_str() {
            "EQUITY" | "ETF" | "INDEX" => ComparisonPeriod::Session,
            "MUTUALFUND" => ComparisonPeriod::ValuationPeriod,
            _ => ComparisonPeriod::Unknown,
        },
        None => ComparisonPeriod::Unknown,
    }
}

/// A price or baseline is only usable when finite and strictly positive.
pub fn sanitize_positive(value: f64) -> Option<f64> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

/// Sanitized quote input for one position. `from_cache` is intentionally not
/// part of the policy: cache freshness is carried by `fetched_at`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionQuote<'a> {
    pub shares: f64,
    pub price: Option<f64>,
    pub previous_close: Option<f64>,
    pub provider_type: Option<&'a str>,
    /// Provider quote time, Unix milliseconds.
    pub quote_time: Option<i64>,
    /// Local fetch time, Unix milliseconds.
    pub fetched_at: Option<i64>,
}

/// Observation quality is separate from the comparison period and cache provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteQuality {
    Available,
    Dated,
    Incomplete,
}

impl QuoteQuality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Dated => "dated",
            Self::Incomplete => "incomplete",
        }
    }
}

/// Policy verdict for one position.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionEvaluation {
    /// Last valid known price times shares; `None` when unpriced.
    pub value: Option<f64>,
    /// UTC date of the quote time, when the position is session-capable,
    /// recent, and priced. Drives same-session bucketing.
    pub session_date: Option<NaiveDate>,
    /// Whether the position can take part in the session aggregate.
    pub session_eligible: bool,
    /// Session move for this position; `None` unless session eligible.
    pub session_amount: Option<f64>,
    pub session_percent: Option<f64>,
    /// Raw price/previous-close comparison for valid price and baseline. It is
    /// exposed separately so a valuation-period move never masquerades as a
    /// session return.
    pub comparison_amount: Option<f64>,
    pub comparison_percent: Option<f64>,
    /// Previous close times shares, the baseline denominator.
    pub comparison_baseline: Option<f64>,
    pub comparison_period: ComparisonPeriod,
    /// UTC date of the quote time, whenever parseable, for presentation.
    pub comparison_date: Option<NaiveDate>,
    pub quality: QuoteQuality,
}

/// Normalize one position against the policy at time `now_ms`.
pub fn evaluate_position(quote: PositionQuote<'_>, now_ms: i64) -> PositionEvaluation {
    let price = quote.price.and_then(sanitize_positive);
    let previous_close = quote.previous_close.and_then(sanitize_positive);
    let shares = sanitize_positive(quote.shares);

    let value = price
        .zip(shares)
        .and_then(|(price, shares)| sanitize_positive(price * shares));
    let baseline = previous_close
        .zip(shares)
        .and_then(|(previous_close, shares)| sanitize_positive(previous_close * shares));

    let comparison = match (value, baseline) {
        (Some(value), Some(baseline)) => {
            let amount = value - baseline;
            let percent = amount / baseline * 100.0;
            (amount.is_finite() && percent.is_finite()).then_some((amount, percent))
        }
        _ => None,
    };
    let (comparison_amount, comparison_percent) = comparison
        .map_or((None, None), |(amount, percent)| {
            (Some(amount), Some(percent))
        });

    let comparison_period = comparison_period(quote.provider_type);
    let quote_recent = quote_time_is_recent(quote.quote_time, now_ms);
    let fetch_recent = fetch_time_is_recent(quote.fetched_at, now_ms);

    // Session bucket membership: session-capable type, valid price, and both
    // timestamps inside the recency budgets. A missing previous close still
    // keeps its date for bucket selection.
    let session_date = if comparison_period == ComparisonPeriod::Session
        && quote_recent
        && fetch_recent
        && value.is_some()
    {
        quote.quote_time.and_then(utc_date)
    } else {
        None
    };

    let session_eligible = session_date.is_some() && comparison_amount.is_some();
    let (session_amount, session_percent) = if session_eligible {
        (comparison_amount, comparison_percent)
    } else {
        (None, None)
    };

    PositionEvaluation {
        value,
        session_date,
        session_eligible,
        session_amount,
        session_percent,
        comparison_amount,
        comparison_percent,
        comparison_baseline: baseline,
        comparison_period,
        comparison_date: quote.quote_time.filter(|ts| *ts > 0).and_then(utc_date),
        quality: if value.is_none()
            || comparison.is_none()
            || comparison_period == ComparisonPeriod::Unknown
            || !timestamp_valid(quote.quote_time, now_ms)
            || !timestamp_valid(quote.fetched_at, now_ms)
        {
            QuoteQuality::Incomplete
        } else if !quote_recent || !fetch_recent {
            QuoteQuality::Dated
        } else {
            QuoteQuality::Available
        },
    }
}

/// Outcome of aggregating positions under the session policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SessionStatus {
    Complete,
    Partial,
    #[default]
    Unavailable,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Session aggregate over one set of positions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionSummary {
    pub amount: Option<f64>,
    pub percent: Option<f64>,
    pub status: SessionStatus,
    pub included_positions: usize,
    pub total_positions: usize,
    /// Latest candidate UTC date; `None` when there is no recent priced session candidate.
    pub reference_date: Option<NaiveDate>,
}

/// Full policy result for a set of positions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PortfolioSummary {
    /// Sum of all valid position values (last valid known prices).
    pub total_value: f64,
    /// Positions without a known valuation, including failed quote requests.
    pub unvalued_positions: usize,
    /// Included value over total priced value, in percent. `None` (unknown)
    /// when any position is unpriced or any requested quote is missing.
    pub coverage_percent: Option<f64>,
    pub session: SessionSummary,
}

/// Aggregate one portfolio (or currency group) under the session policy.
///
/// `missing_quotes` counts positions whose quote could not be fetched at all;
/// they count towards totals and force coverage to unknown, but they carry no
/// currency and are therefore not part of any group's items.
pub fn aggregate(evaluations: &[PositionEvaluation], missing_quotes: usize) -> PortfolioSummary {
    let total_positions = evaluations.len() + missing_quotes;
    let total_value: f64 = evaluations.iter().filter_map(|e| e.value).sum();

    // Latest UTC quote date among valid, recent, session-capable valuations
    // (positions missing a previous close still nominate their date).
    let reference_date = evaluations.iter().filter_map(|e| e.session_date).max();
    let included: Vec<&PositionEvaluation> = evaluations
        .iter()
        .filter(|e| e.session_eligible && e.session_date == reference_date)
        .collect();

    let session_baseline: f64 = included.iter().filter_map(|e| e.comparison_baseline).sum();
    let session_amount = (!included.is_empty())
        .then(|| {
            included
                .iter()
                .filter_map(|e| e.session_amount)
                .sum::<f64>()
        })
        .filter(|amount| amount.is_finite() && session_baseline.is_finite());
    let session_percent = session_amount
        .filter(|_| session_baseline > 0.0)
        .map(|amount| amount / session_baseline * 100.0)
        .filter(|percent| percent.is_finite());
    let session_amount = session_amount.filter(|_| session_percent.is_some());
    let included_positions = if session_amount.is_some() {
        included.len()
    } else {
        0
    };

    let status = if included_positions == 0 {
        SessionStatus::Unavailable
    } else if included_positions < total_positions {
        SessionStatus::Partial
    } else {
        SessionStatus::Complete
    };

    let included_value: f64 = if included_positions == 0 {
        0.0
    } else {
        included.iter().filter_map(|e| e.value).sum()
    };
    let coverage_percent = if total_positions == 0
        || missing_quotes > 0
        || !included_value.is_finite()
        || evaluations.iter().any(|e| e.value.is_none())
    {
        None
    } else {
        sanitize_positive(total_value).map(|total| included_value / total * 100.0)
    };

    PortfolioSummary {
        total_value,
        unvalued_positions: evaluations.iter().filter(|e| e.value.is_none()).count()
            + missing_quotes,
        coverage_percent,
        session: SessionSummary {
            amount: session_amount,
            percent: session_percent,
            status,
            included_positions,
            total_positions,
            reference_date,
        },
    }
}

fn utc_date(timestamp_ms: i64) -> Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms).map(|dt| dt.date_naive())
}

fn timestamp_valid(timestamp: Option<i64>, now_ms: i64) -> bool {
    // Positive, and at most MAX_FUTURE_MS ahead of `now_ms` (clock skew).
    timestamp.is_some_and(|ts| ts > 0 && ts.saturating_sub(now_ms) <= MAX_FUTURE_MS)
}

fn quote_time_is_recent(quote_time: Option<i64>, now_ms: i64) -> bool {
    timestamp_valid(quote_time, now_ms)
        && quote_time.is_some_and(|ts| now_ms.saturating_sub(ts) <= MAX_QUOTE_AGE_MS)
}

fn fetch_time_is_recent(fetched_at: Option<i64>, now_ms: i64) -> bool {
    timestamp_valid(fetched_at, now_ms)
        && fetched_at.is_some_and(|ts| now_ms.saturating_sub(ts) <= MAX_FETCH_AGE_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_789_387_200_000;
    const TWO_HOURS_MS: i64 = 7_200_000;
    const FIVE_MINUTES_MS: i64 = 300_000;
    const NINETY_SIX_HOURS_MS: i64 = 96 * 60 * 60 * 1000;
    const TWO_MINUTES_MS: i64 = 120_000;

    fn evaluate(
        price: Option<f64>,
        previous_close: Option<f64>,
        provider_type: Option<&str>,
        quote_time: Option<i64>,
        fetched_at: Option<i64>,
    ) -> PositionEvaluation {
        evaluate_position(
            PositionQuote {
                shares: 10.0,
                price,
                previous_close,
                provider_type,
                quote_time,
                fetched_at,
            },
            NOW,
        )
    }

    fn eligible_etf() -> PositionEvaluation {
        evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW),
        )
    }

    #[test]
    fn sanitize_positive_rejects_unusable_values() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(sanitize_positive(value), None, "{value}");
        }
        assert_eq!(sanitize_positive(1e-12), Some(1e-12));
    }

    #[test]
    fn comparison_period_classifies_provider_types() {
        for provider_type in ["EQUITY", "ETF", "INDEX", "etf"] {
            assert_eq!(
                comparison_period(Some(provider_type)),
                ComparisonPeriod::Session,
                "{provider_type}"
            );
        }
        assert_eq!(
            comparison_period(Some("MUTUALFUND")),
            ComparisonPeriod::ValuationPeriod
        );
        assert_eq!(
            comparison_period(Some("CRYPTOCURRENCY")),
            ComparisonPeriod::Unknown
        );
        assert_eq!(comparison_period(None), ComparisonPeriod::Unknown);
    }

    #[test]
    fn nonfinite_or_nonpositive_prices_have_no_value() {
        for price in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            let evaluation = evaluate(
                Some(price),
                Some(95.0),
                Some("ETF"),
                Some(NOW - TWO_HOURS_MS),
                Some(NOW),
            );
            assert_eq!(evaluation.value, None, "price {price}");
            assert!(!evaluation.session_eligible, "price {price}");
        }
    }

    #[test]
    fn zero_or_negative_baseline_has_no_comparison() {
        for previous_close in [0.0, -95.0, f64::NAN] {
            let evaluation = evaluate(
                Some(100.0),
                Some(previous_close),
                Some("ETF"),
                Some(NOW - TWO_HOURS_MS),
                Some(NOW),
            );
            assert_eq!(evaluation.value, Some(1000.0));
            assert_eq!(evaluation.comparison_amount, None);
            assert!(!evaluation.session_eligible);
        }
    }

    #[test]
    fn timestamps_too_far_in_the_future_are_rejected() {
        let future = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW + FIVE_MINUTES_MS),
            Some(NOW),
        );
        assert!(future.session_eligible);

        let too_future = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW + FIVE_MINUTES_MS + 1),
            Some(NOW),
        );
        assert!(!too_future.session_eligible);
        assert_eq!(too_future.value, Some(1000.0));

        let future_fetch = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW + FIVE_MINUTES_MS + 1),
        );
        assert!(!future_fetch.session_eligible);
    }

    #[test]
    fn missing_timestamps_keep_valuation_but_lose_session() {
        let no_quote_time = evaluate(Some(100.0), Some(95.0), Some("ETF"), None, Some(NOW));
        assert_eq!(no_quote_time.value, Some(1000.0));
        assert_eq!(no_quote_time.comparison_amount, Some(50.0));
        assert!(!no_quote_time.session_eligible);
        assert_eq!(no_quote_time.comparison_date, None);

        let no_fetch_time = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            None,
        );
        assert_eq!(no_fetch_time.value, Some(1000.0));
        assert!(!no_fetch_time.session_eligible);
    }

    #[test]
    fn stale_quote_or_fetch_time_loses_session() {
        let stale_quote = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - NINETY_SIX_HOURS_MS - 1),
            Some(NOW),
        );
        assert!(!stale_quote.session_eligible);
        assert_eq!(stale_quote.value, Some(1000.0));
        assert_eq!(stale_quote.comparison_amount, Some(50.0));

        let boundary_quote = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - NINETY_SIX_HOURS_MS),
            Some(NOW),
        );
        assert!(boundary_quote.session_eligible);

        let stale_fetch = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW - TWO_MINUTES_MS - 1),
        );
        assert!(!stale_fetch.session_eligible);

        let boundary_fetch = evaluate(
            Some(100.0),
            Some(95.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW - TWO_MINUTES_MS),
        );
        assert!(boundary_fetch.session_eligible);
    }

    #[test]
    fn repeated_identical_fresh_prints_are_all_included() {
        let evaluations = vec![eligible_etf(), eligible_etf()];
        let summary = aggregate(&evaluations, 0);

        assert_eq!(summary.session.status, SessionStatus::Complete);
        assert_eq!(summary.session.included_positions, 2);
        assert!((summary.session.amount.unwrap() - 100.0).abs() < 1e-9);
        assert_eq!(summary.coverage_percent, Some(100.0));
    }

    #[test]
    fn periodic_fund_keeps_valuation_comparison_without_session() {
        let fund = evaluate(
            Some(21.0),
            Some(20.0),
            Some("MUTUALFUND"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW),
        );
        assert_eq!(fund.comparison_period, ComparisonPeriod::ValuationPeriod);
        assert_eq!(fund.value, Some(210.0));
        assert!(!fund.session_eligible);
        assert_eq!(fund.session_amount, None);
        assert_eq!(fund.comparison_amount, Some(10.0));
        assert!(fund.comparison_date.is_some());

        let summary = aggregate(&[fund], 0);
        assert_eq!(summary.session.status, SessionStatus::Unavailable);
        assert_eq!(summary.session.amount, None);
        assert_eq!(summary.coverage_percent, Some(0.0));
    }

    #[test]
    fn missing_previous_close_is_unavailable_not_zero() {
        let incomplete = evaluate(
            Some(20.0),
            None,
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW),
        );
        assert_eq!(incomplete.value, Some(200.0));
        assert!(incomplete.session_date.is_some());

        let summary = aggregate(&[incomplete], 0);
        assert_eq!(summary.session.amount, None);
        assert_eq!(summary.session.percent, None);
        assert_eq!(summary.session.status, SessionStatus::Unavailable);
        assert_eq!(summary.coverage_percent, Some(0.0));
    }

    #[test]
    fn aggregate_selects_latest_session_date_only() {
        let current = eligible_etf();
        let older = evaluate(
            Some(100.0),
            Some(90.0),
            Some("ETF"),
            Some(NOW - 3 * 24 * 60 * 60 * 1000),
            Some(NOW),
        );

        let summary = aggregate(&[current, older], 0);
        assert_eq!(summary.session.status, SessionStatus::Partial);
        assert_eq!(summary.session.included_positions, 1);
        assert!((summary.session.amount.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(summary.total_value, 2000.0);
        assert_eq!(summary.coverage_percent, Some(50.0));
    }

    #[test]
    fn genuinely_flat_session_is_a_real_zero() {
        let flat = evaluate(
            Some(20.0),
            Some(20.0),
            Some("EQUITY"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW),
        );
        assert!(flat.session_eligible);
        assert_eq!(flat.session_amount, Some(0.0));
        assert_eq!(flat.session_percent, Some(0.0));

        let summary = aggregate(&[flat], 0);
        assert_eq!(summary.session.amount, Some(0.0));
        assert_eq!(summary.session.percent, Some(0.0));
        assert_eq!(summary.session.status, SessionStatus::Complete);
        assert_eq!(summary.coverage_percent, Some(100.0));
    }

    #[test]
    fn empty_portfolio_has_no_invented_numbers() {
        let summary = aggregate(&[], 0);
        assert_eq!(summary.total_value, 0.0);
        assert_eq!(summary.session.amount, None);
        assert_eq!(summary.session.percent, None);
        assert_eq!(summary.session.status, SessionStatus::Unavailable);
        assert_eq!(summary.coverage_percent, None);
    }

    #[test]
    fn missing_quote_makes_coverage_unknown() {
        let summary = aggregate(&[eligible_etf()], 1);
        assert_eq!(summary.coverage_percent, None);
        assert_eq!(summary.session.status, SessionStatus::Partial);
        assert_eq!(summary.session.total_positions, 2);
        assert_eq!(summary.session.included_positions, 1);
    }

    #[test]
    fn aggregate_overflow_never_becomes_infinite_pnl() {
        let position = evaluate(Some(1e307), Some(1e306), Some("ETF"), Some(NOW), Some(NOW));
        let summary = aggregate(&[position.clone(), position.clone(), position], 0);
        assert_eq!(summary.session.amount, None);
        assert_eq!(summary.session.percent, None);
        assert_eq!(summary.session.status, SessionStatus::Unavailable);
        assert_eq!(summary.coverage_percent, None);
    }

    #[test]
    fn latest_incomplete_session_retains_reference_date() {
        let evaluation = evaluate(Some(100.0), None, Some("ETF"), Some(NOW), Some(NOW));
        let summary = aggregate(&[evaluation], 0);
        assert_eq!(summary.session.reference_date, utc_date(NOW));
        assert_eq!(summary.session.percent, None);
    }

    #[test]
    fn unpriced_position_makes_coverage_unknown() {
        let unpriced = evaluate(
            None,
            Some(20.0),
            Some("ETF"),
            Some(NOW - TWO_HOURS_MS),
            Some(NOW),
        );
        let summary = aggregate(&[eligible_etf(), unpriced], 0);

        assert_eq!(summary.total_value, 1000.0);
        assert_eq!(summary.coverage_percent, None);
        assert_eq!(summary.session.status, SessionStatus::Partial);
    }
}
