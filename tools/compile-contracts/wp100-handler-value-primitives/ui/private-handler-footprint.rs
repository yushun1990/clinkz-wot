use clinkz_wot_core::HandlerFootprint;

fn main() {
    let _ = HandlerFootprint {
        retained_bytes: 1,
        pending_call_bytes: 2,
        subscription_bytes: 3,
    };
}
