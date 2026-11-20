use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const DISPATCH_OFFSET: usize = 0x384FA10;
const EXPECTED: [u32; 4] = [0xA9BC5FF8, 0xA90157F6, 0xA9024FF4, 0xA9037BFD];

static APPLIED: AtomicU32 = AtomicU32::new(0);
static INVALID_STATE_REPORTED: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = 0x384FA10)]
unsafe fn advance_dispatch(state: u64, mode: u32) {
    if state != 0 {
        let base = state as *mut u8;
        let initialized = std::ptr::read_volatile(base.add(0x1d18));

        if initialized == 1 {
            let current = std::ptr::read_volatile(base.add(0x1d20).cast::<u32>());
            let queued = std::ptr::read_volatile(base.add(0x1d30).cast::<u64>());

            if current <= 1 && queued <= 1 {
                std::ptr::write_volatile(base.add(0x1d30).cast::<u64>(), (current ^ 1) as u64);
                std::ptr::write_volatile(base.add(0x1d28), 1);

                match APPLIED.fetch_add(1, Ordering::Relaxed) {
                    0 => skyline::println!("[remove-the-frame] Test dispatch actif : premiere anticipation"),
                    119 => skyline::println!("[remove-the-frame] Test dispatch actif : 120 anticipations"),
                    _ => {}
                }
            } else if !INVALID_STATE_REPORTED.swap(true, Ordering::Relaxed) {
                skyline::println!(
                    "[remove-the-frame] Dispatch : index hors plage, anticipation ignoree"
                );
            }
        }
    }

    call_original!(state, mode)
}

pub fn install() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let actual = unsafe {
        [
            std::ptr::read_volatile(base.add(DISPATCH_OFFSET / 4)),
            std::ptr::read_volatile(base.add(DISPATCH_OFFSET / 4 + 1)),
            std::ptr::read_volatile(base.add(DISPATCH_OFFSET / 4 + 2)),
            std::ptr::read_volatile(base.add(DISPATCH_OFFSET / 4 + 3)),
        ]
    };

    if actual != EXPECTED {
        skyline::println!("[remove-the-frame] Dispatch : signature inconnue, test inactif");
        return;
    }

    skyline::install_hook!(advance_dispatch);
    skyline::println!("[remove-the-frame] Test dispatch actif installe (VSync et buffers inchanges)");
}
