use crate::codegen;

/// Use this when a feature is not supported. Default implementation
pub struct Unimplemented;

impl codegen::ACFCodegen for Unimplemented {
    fn alloc_label(&mut self) -> codegen::Label {
        unimplemented!("arbitrary control flow is not supported by this backend");
    }

    fn label(&mut self, _: codegen::Label) {
        unimplemented!("arbitrary control flow is not supported by this backend");
    }

    fn jump(&mut self, _: codegen::Label) {
        unimplemented!("arbitrary control flow is not supported by this backend");
    }

    fn cjump(&mut self, _: codegen::Value, _: codegen::Label) {
        unimplemented!("arbitrary control flow is not supported by this backend");
    }
}

impl codegen::Intrinsics for Unimplemented {}
