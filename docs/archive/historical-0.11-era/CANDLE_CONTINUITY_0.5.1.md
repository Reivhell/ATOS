# Candle continuity contract — v0.5.1

## Problem

The previous market-data boundary compared the median adjacent timestamp delta with the requested interval. Median validation can accept a series containing a missing candle if most rows remain regularly spaced. Indicators and research detectors may then consume a discontinuous series without an explicit data-quality failure.

## Contract

For the currently configured continuous-market series, each adjacent candle timestamp must differ by exactly the requested interval:

- timestamps remain strictly increasing and safe integers;
- each delta must equal the configured interval in milliseconds;
- a gap rejects the entire response before any chart or indicator consumes it;
- the error identifies the row, expected delta, observed delta, and number of missing intervals when that count is integral;
- there is no silent interpolation, candle synthesis, or partial-series return.

## Scope and limitations

This contract is intentionally strict and appropriate for the current continuous-market crypto-series boundary. It must not be applied unchanged to instruments with scheduled sessions, holidays, or venue-specific trading breaks. Supporting those markets requires a trusted session calendar and an explicit expected-cadence policy. The validator does not prove provider authenticity and is not a substitute for signed provider data or server-side provenance.

## Verification

`npm run test:market-data` exercises valid cadence, wrong interval, a missing interval, irregular timestamp offsets, non-monotonic timestamps, malformed OHLCV, stale/future timestamps, and request mismatch. `npm test` and `npm run verify:repo` are required for the release candidate. Frontend and native Rust build requirements remain separate release gates.
