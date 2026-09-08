use sea_orm::{ConnectionTrait, Cursor, DbErr, SelectorTrait};
use std::env;

/// Hard ceiling on a single keyset page. Requests above this are clamped.
/// Override with the `MAX_PAGE_LIMIT` env var.
pub fn max_page_limit() -> u64 {
	env_u64("MAX_PAGE_LIMIT", 2000)
}

/// Page size used when the caller omits `limit` or sends `0`. Override with the
/// `DEFAULT_PAGE_LIMIT` env var.
pub fn default_page_limit() -> u64 {
	env_u64("DEFAULT_PAGE_LIMIT", 100)
}

fn env_u64(key: &str, default: u64) -> u64 {
	env::var(key)
		.ok()
		.and_then(|v| v.trim().parse().ok())
		.unwrap_or(default)
}

/// Clamp a requested page limit into `[1, max_page_limit()]`, substituting
/// [`default_page_limit`] for an absent or zero value. Out-of-range values are
/// clamped rather than rejected so a list endpoint never 400s on `limit` alone.
pub fn clamp_page_limit(requested: Option<u64>) -> u64 {
	match requested {
		None | Some(0) => default_page_limit(),
		Some(n) => n.min(max_page_limit()),
	}
}

#[derive(Debug, Clone)]
pub struct KeysetPage<T> {
	pub rows: Vec<T>,
	pub has_more: bool,
}

impl<T> KeysetPage<T> {
	pub fn map_rows<U, F: FnMut(T) -> U>(self, f: F) -> KeysetPage<U> {
		KeysetPage {
			rows: self.rows.into_iter().map(f).collect(),
			has_more: self.has_more,
		}
	}
}

/// Fetch one keyset page from an already-positioned [`Cursor`].
///
/// The caller is responsible for the `cursor_by(...)` ordering tuple and any
/// `after(...)` seeking. This applies the clamp and the N+1 trick: it requests
/// `limit + 1` rows, infers `has_more` from the overflow row, then truncates so
/// the page never exceeds `limit`. No `COUNT(*)` is issued.
pub async fn fetch_keyset_page<S, C>(
	cursor: &mut Cursor<S>,
	requested_limit: Option<u64>,
	conn: &C,
) -> Result<KeysetPage<S::Item>, DbErr>
where
	S: SelectorTrait,
	C: ConnectionTrait,
{
	let limit = clamp_page_limit(requested_limit);
	let mut rows = cursor.first(limit + 1).all(conn).await?;
	let has_more = rows.len() as u64 > limit;
	if has_more {
		rows.truncate(limit as usize);
	}
	Ok(KeysetPage { rows, has_more })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_substitutes_default_for_absent_or_zero() {
		assert_eq!(clamp_page_limit(None), default_page_limit());
		assert_eq!(clamp_page_limit(Some(0)), default_page_limit());
	}

	#[test]
	fn clamp_caps_at_max_and_passes_in_range() {
		assert_eq!(clamp_page_limit(Some(1)), 1);
		assert_eq!(clamp_page_limit(Some(25)), 25);
		assert_eq!(clamp_page_limit(Some(50)), 50);
		assert_eq!(
			clamp_page_limit(Some(max_page_limit() + 1)),
			max_page_limit()
		);
		assert_eq!(clamp_page_limit(Some(u64::MAX)), max_page_limit());
	}
}
