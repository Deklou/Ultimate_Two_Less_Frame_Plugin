use std::sync::atomic::{AtomicU32, Ordering};

const RENDER_OFFSET: usize = 0x386B100;
const RENDER_EXPECTED: [u32; 4] = [0x5295556A, 0x72B5554A, 0x9BAA7D2A, 0xD361FD4A];

static SAMPLES: AtomicU32 = AtomicU32::new(0);
static MOD_ZERO: AtomicU32 = AtomicU32::new(0);
static MOD_ONE: AtomicU32 = AtomicU32::new(0);
static MOD_TWO: AtomicU32 = AtomicU32::new(0);
static MAX_RAW: AtomicU32 = AtomicU32::new(0);
static LAST_RAW: AtomicU32 = AtomicU32::new(u32::MAX);

pub fn snapshot() -> (u32, u32) {
    (
        SAMPLES.load(Ordering::Acquire),
        LAST_RAW.load(Ordering::Acquire),
    )
}

#[skyline::hook(offset = 0x386B100, inline)]
unsafe fn observe_render_index(ctx: &skyline::hooks::InlineCtx) {
    crate::pipeline_trace::on_render();
    let raw = ctx.registers[9].w();
    let selected = raw % 3;
    LAST_RAW.store(selected, Ordering::Release);
    let n = SAMPLES.fetch_add(1, Ordering::Relaxed);
    if n >= 120 {
        return;
    }

    MAX_RAW.fetch_max(raw, Ordering::Relaxed);
    match selected {
        0 => { MOD_ZERO.fetch_add(1, Ordering::Relaxed); }
        1 => { MOD_ONE.fetch_add(1, Ordering::Relaxed); }
        _ => { MOD_TWO.fetch_add(1, Ordering::Relaxed); }
    }

    if n == 119 {
        skyline::println!(
            "[remove-the-frame] Rendu apres addition : 120 index, mod3=[{}, {}, {}], max_avant_mod={}",
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
