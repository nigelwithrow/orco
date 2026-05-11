pub(super) struct ValueInfo {
    /// Expression for this value, will either be placed whenever
    /// the value is used or placed in the code whenever the value is flushed.
    pub(super) expression: String,
    pub(super) ty: orco::Type,
}
