use clinkz_wot_core::AsyncReadPropertyHandler;

fn main() {
    let _ = core::mem::size_of::<&dyn AsyncReadPropertyHandler>();
}
