/// `Stale`/`Unavailable` will be produced by the service once real collectors
/// start reporting errors. Marked `allow(dead_code)` until then.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    Fresh,
    Stale,
    Unavailable,
}
