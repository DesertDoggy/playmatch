use std::env;

/// Hard cap on the number of items a single bulk request may carry. Uniform
/// across every bulk endpoint so callers learn one limit. A bulk request still
/// counts as one ordinary request against the per-IP rate limiter regardless of
/// how many items it carries. Override with the `MAX_BULK_ITEMS` env var.
pub fn max_bulk_items() -> usize {
	env_usize("MAX_BULK_ITEMS", 5000)
}

/// Upper bound on per-item lookups run in parallel within a single bulk batch.
/// Keeps fan-out from a single batch bounded against the shared database and
/// cache pools. Override with the `BULK_CONCURRENCY` env var.
pub fn bulk_concurrency() -> usize {
	env_usize("BULK_CONCURRENCY", 64)
}

fn env_usize(key: &str, default: usize) -> usize {
	env::var(key)
		.ok()
		.and_then(|v| v.trim().parse().ok())
		.unwrap_or(default)
}
