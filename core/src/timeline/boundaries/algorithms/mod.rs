/// Type-based boundary detection algorithms
///
/// Each algorithm normalizes a different data type:
/// - Continuous: infinitesimal signals → statistical changepoints (PELT)
/// - Discrete: point events → temporal sessions (gap clustering)
/// - Interval: pre-defined spans → direct extraction

pub mod continuous;
pub mod discrete;
pub mod interval;
