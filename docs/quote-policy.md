# Quote and P&L policy

This document states how fincli turns raw Yahoo Finance quotes into portfolio
valuations and P&L figures. The rules live in `src/quote_policy.rs` as pure
functions with an injected `now`; the Yahoo adapter (`src/finance.rs`) only
normalizes provider-specific data into that shape.

The two headline rules:

- **Total valuation** includes every position with a last valid known price,
  even when that price is stale or the position has no comparable baseline.
- **Session P&L** includes only positions that are comparable and eligible on
  the same session date. Everything unknown is shown as `N/A`, never as a
  fabricated zero.

## Data contract

- `price` and `previousClose` are only usable when finite and strictly
  positive. Missing, zero, negative, or non-finite values become `null`
  (`None`).
- The provider quote type (`instrumentType`, falling back to `quoteType`) and
  the quote time are retained next to the local fetch time; they are separate
  concepts and never overwrite each other.
- Yahoo reports `regularMarketTime` in seconds; the adapter converts every
  timestamp to Unix milliseconds. All internal timestamps are milliseconds.
- `fromCache` only describes where the data came from. Quote freshness is
  judged from the quote and fetch timestamps, not from cache state.
- A missing `previousClose` is never replaced by the current price, and
  `chartPreviousClose` is never used as an automatic fallback: it is
  range-dependent and would fabricate a baseline.

## Comparison periods

The comparison period is derived from the provider type in the pure
normalizer, not from per-ticker UI rules and not from any configured asset
class:

| Provider type | Period |
| --- | --- |
| `EQUITY`, `ETF`, `INDEX` | session-capable |
| `MUTUALFUND` | valuation period (never session-eligible, even on publication day) |
| absent or anything else | unknown |

For any position with a valid price and baseline, fincli exposes the raw
comparison amount and percent together with its period and source date, so a
valuation-period move can never be presented as a session return.

## Recency budgets

A position is eligible for session P&L only when:

- the quote time and fetch time are positive, and at most 5 minutes in the
  future (clock-skew tolerance);
- the quote time is no older than 96 hours;
- the fetch time is no older than 120 seconds.

These are deliberately conservative *recency budgets*, not a trading calendar
and not a measured NAV publication frequency: fincli does not know exchange
holidays or fund publication schedules. A cached quote within these budgets
can be eligible; an old fallback cache entry keeps its price for valuation but
is excluded from session P&L. A freshly fetched but dated NAV stays dated.
Consequences worth knowing:

- on a weekend, a Friday session quote is still recent and remains eligible;
- a fund whose provider publishes one NAV per day can still be compared as a
  valuation-period move, but never becomes a session return;
- a genuinely unchanged fresh price is a real `0.00%` and is eligible.

## Session aggregation

1. Among valid, recent, session-capable valuations (including ones missing a
   previous close), select the latest **UTC** quote date.
2. Include only eligible comparisons on that date. Older-date rows are
   excluded, so a Friday US close is never combined with a Monday EU session.
3. The session amount is the sum of `(price - previousClose) * shares` over
   the included subset, and the percent uses the **same subset's** baseline
   (`previousClose * shares`) as denominator.

UTC date-bucketing is an explicit convention, not an exchange-calendar
guarantee. The session result carries a nullable amount/percent, a status
(`complete`, `partial`, or `unavailable`), included/total position counts, and
the reference session date. Output is labeled "Session P&L" with that
reference date — never "today" or "24h".

## Coverage

Coverage = included position value / total priced value, in percent. It is
`N/A` (unknown) whenever any position has an unknown valuation or a requested
quote is missing — a full-portfolio percentage is then never claimed, in table
mode, `--total` mode, and per-currency subtotals alike. Unpriced positions
keep their rows and produce warnings instead of disappearing silently. Valuation
headlines are explicitly priced subtotals when any requested position is unvalued;
if none is valued, the displayed amount is `N/A`, not zero.

Failed quotes have unknown currency. They conservatively count as unvalued in
every currency group's coverage (group counts include these unassigned requests);
no group can claim completeness just because the failed ticker was omitted.

## Currencies

Positions are grouped per quote currency and subtotals never sum unlike
currencies. A missing currency makes the position's monetary valuation unknown.

## Caching

Cache entries round-trip the source quote date and provider type. Legacy
cache entries that predate these metadata fields deserialize safely but cannot
acquire invented dates or eligibility: they fail closed (valuation retained,
session P&L `N/A`). Invalid cache timestamps are never replaced with the current
time. Network failures or invalid fresh prices can reuse a last valid cached
valuation without refreshing its original dates.

Every portfolio row shows its quoted date, period and observation quality,
including missing-baseline rows. Off-reference-date rows have `N/A` in the
aggregate session column. Their separate comparison, or `N/A comparison`,
remains visible with source metadata in both normal and `--total` modes.

## What this is not

- Not cost-basis or cumulative P&L: figures compare the latest price against
  the provider's previous close (or previous NAV) only.
- Not an FX engine: unlike currencies are grouped, never converted.
- Not a trading calendar: see the recency budgets above.
