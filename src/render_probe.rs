use std::sync::atomic::{AtomicU32, Ordering};

const RENDER_OFFSET: usize = 0x386B0FC;
const RENDER_EXPECTED: [u32; 4] = [0x11000929, 0x5295556A, 0x72B5554A, 0x9BAA7D2A];

static SAMPLES: AtomicU32 = AtomicU32::new(0);
static MOD_ZERO: AtomicU32 = AtomicU32::new(0);
static MOD_ONE: AtomicU32 = AtomicU32::new(0);
static MOD_TWO: AtomicU32 = AtomicU32::new(0);
static MAX_RAW: AtomicU32 = AtomicU32::new(0);

#[skyline::hook(offset = 0x386B0FC, inline)]
unsafe fn observe_render_index(ctx: &skyline::hooks::InlineCtx) {
    let raw = ctx.registers[9].w();
    let n = SAMPLES.fetch_add(1, Ordering::Relaxed);
    if n >= 120 {
        return;
    }

    MAX_RAW.fetch_max(raw, Ordering::Relaxed);
    match raw % 3 {
        0 => { MOD_ZERO.fetch_add(1, Ordering::Relaxed); }
        1 => { MOD_ONE.fetch_add(1, Ordering::Relaxed); }
        _ => { MOD_TWO.fetch_add(1, Ordering::Relaxed); }
    }

    if n == 119 {
        skyline::println!(
            "[remove-the-frame] Rendu : 120 index, mod3=[{}, {}, {}], max_brut={}",
            MOD_ZERO.load(Ordering::Relaxed),
            MOD_ONE.load(Ordering::Relaxed),
            MOD_TWO.load(Ordering::Relaxed),
            MAX_RAW.load(Ordering::Relaxed),
        );
    }
}

pub fn install() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let actual = unsafe {
        [
            std::ptr::read_volatile(base.add(RENDER_OFFSET / 4)),
            std::ptr::read_volatile(base.add(RENDER_OFFSET / 4 + 1)),
            std::ptr::read_volatile(base.add(RENDER_OFFSET / 4 + 2)),
            std::ptr::read_volatile(base.add(RENDER_OFFSET / 4 + 3)),
        ]
    };

    if actual != RENDER_EXPECTED {
        skyline::println!("[remove-the-frame] Rendu : signature inconnue, sonde inactive");
        return;
    }

    skyline::install_hook!(observe_render_index);
    skyline::println!("[remove-the-frame] Sonde index rendu installee (aucun changement)");
}
