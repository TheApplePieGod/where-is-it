#[cfg(feature = "node-compile")]
extern crate napi_build;

pub fn main() {
    #[cfg(feature = "node-compile")]
    napi_build::setup();
}
