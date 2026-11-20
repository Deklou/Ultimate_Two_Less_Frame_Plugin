use std::sync::atomic::{AtomicU32, Ordering};

const PRESENT_OFFSET: usize = 0x3748128;
const PRESENT_EXPECTED: [u32; 2] = [0x9404A02E, 0xB000DF88];

static COUNT: AtomicU32 = AtomicU32::new(0);
static SLOT_0: AtomicU32 = AtomicU32::new(0);
static SLOT_1: AtomicU32 = AtomicU32::new(0);
static SLOT_2: AtomicU32 = AtomicU32::new(0);
static INVALID: AtomicU32 = AtomicU32::new(0);
static DELTA_0: AtomicU32 = AtomicU32::new(0);
static DELTA_1: AtomicU32 = AtomicU32::new(0);
static DELTA_2: AtomicU32 = AtomicU32::new(0);
static WITHOUT_RENDER: AtomicU32 = AtomicU32::new(0);

#[skyline::hook(offset = 0x3748128, inline)]
unsafe fn observe_present(ctx: &skyline::hooks::InlineCtx) {
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    if n >= 120 {
        return;
    }

    // x1 pointe vers la structure lue par FUN_71038701e0.
    let state = ctx.registers[1].x() as *const u8;
    if state.is_null() {
        INVALID.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let slot = std::ptr::read_volatile(state.add(0xf0).cast::<u32>());
    match slot {
        0 => { SLOT_0.fetch_add(1, Ordering::Relaxed); }
        1 => { SLOT_1.fetch_add(1, Ordering::Relaxed); }
        2 => { SLOT_2.fetch_add(1, Ordering::Relaxed); }
        _ => { INVALID.fetch_add(1, Ordering::Relaxed); }
    }

    let (render_count, render_index) = crate::render_probe::snapshot();
    if render_count == 0 || render_index > 2 || slot > 2 {
        WITHOUT_RENDER.fetch_add(1, Ordering::Relaxed);
    } else {
        match (slot + 3 - render_index) % 3 {
            0 => { DELTA_0.fetch_add(1, Ordering::Relaxed); }
            1 => { DELTA_1.fetch_add(1, Ordering::Relaxed); }
            _ => { DELTA_2.fetch_add(1, Ordering::Relaxed); }
        }
    }

    if n == 119 {
        skyline::println!(
            "[remove-the-frame] Present : 120 appels, slot_f0=[{}, {}, {}], delta=[{}, {}, {}], sans_correspondance={}, invalides={}",
            SLOT_0.load(Ordering::Relaxed),
            SLOT_1.load(Ordering::Relaxed),
            SLOT_2.load(Ordering::Relaxed),
            DELTA_0.load(Ordering::Relaxed),
            DELTA_1.load(Ordering::Relaxed),
            DELTA_2.load(Ordering::Relaxed),
            WITHOUT_RENDER.load(Ordering::Relaxed),
            INVALID.load(Ordering::Relaxed),
        );
    }
}

pub fn install() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let observed = unsafe {
        [
            std::ptr::read_volatile(base.add(PRESENT_OFFSET / 4)),
            std::ptr::read_volatile(base.add(PRESENT_OFFSET / 4 + 1)),
        ]
    };
    if observed != PRESENT_EXPECTED {
        skyline::println!("[remove-the-frame] Present : signature inconnue, sonde inactive");
        return;
    }

    skyline::install_hook!(observe_present);
    skyline::println!("[remove-the-frame] Sonde de presentation installee (aucun changement)");
}
