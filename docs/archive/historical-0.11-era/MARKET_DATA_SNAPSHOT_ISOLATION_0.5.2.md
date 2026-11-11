# Immutable validated market-data snapshots — v0.5.2

## Risk addressed

The previous UI path validated a Tauri bridge response and then continued to render the original response object. Validation alone does not prevent the object from changing afterward. That leaves a time-of-check/time-of-use (TOCTOU) gap between checks and downstream chart/indicator reads.

## Contract

`validateCandleSeriesResponse` now returns a newly allocated snapshot containing only the known response metadata and OHLCV fields. The request, each candle, the candle array, and the top-level response are frozen. The UI assigns and returns this validated snapshot rather than retaining the raw bridge response.

This provides in-process mutation isolation after validation and reduces the data surface exposed to downstream components. It does not make the JavaScript frontend an authorization boundary, authenticate the upstream provider, defend against a compromised runtime, or replace native validation.

## Regression coverage

- The validator returns a detached object rather than the input reference.
- Nested objects and arrays are frozen.
- Mutating the bridge payload after validation cannot change the accepted prices, series length, request identity, or source metadata.
- Attempts to mutate the validated snapshot throw in strict-mode tests.

## Release boundary

Node tests and static repository checks are necessary but insufficient for production certification. The frontend build, native Rust tests/Clippy, pinned dependency resolution, and platform package acceptance still need to pass in CI.
