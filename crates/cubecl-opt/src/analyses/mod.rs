mod base;
pub mod dominance;
// Upstream analysis module retained verbatim so future rebases stay clean.
// Nothing in cubecl currently consumes it (its own `Ranges` already carries
// `#[allow(unused)]`), and the fork lints with `-D warnings`.
#[allow(dead_code)]
pub mod integer_range;
pub mod liveness;
pub mod post_order;
pub mod uniformity;
pub mod writes;

pub use base::*;
