mod render_probe;
mod present_probe;
mod pipeline_trace;
mod dispatch_ready;

use std::sync::atomic::{AtomicU32, Ordering};

const VSYNC_OFFSET: usize = 0x3811014;
const VSYNC_EXPECTED: [u32; 2] = [0xC85FFEA8, 0x91000508];

static VSYNC_COUNT: AtomicU32 = AtomicU32::new(0);

#[skyline::hook(offset = 0x3811014, inline)]
unsafe fn observe_vsync(_: &skyline::hooks::InlineCtx) {
    if VSYNC_COUNT.fetch_add(1, Ordering::Relaxed) == 119 {
        skyline::println!("[remove-the-frame] VSync : 120 passages observes");
    }
}

#[skyline::main(name = "remove-the-frame")]
pub fn main() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let observed = unsafe {
        [
            std::ptr::read_volatile(base.add(VSYNC_OFFSET / 4)),
            std::ptr::read_volatile(base.add(VSYNC_OFFSET / 4 + 1)),
        ]
    };

    if observed != VSYNC_EXPECTED {
        skyline::println!("[remove-the-frame] VSync : signature inconnue, hook non installe");
        return;
    }

    skyline::install_hook!(observe_vsync);
    skyline::println!("[remove-the-frame] Sonde VSync installee (timing inchange)");
    render_probe::install();
    present_probe::install();
    pipeline_trace::install();
    dispatch_ready::install();
}
