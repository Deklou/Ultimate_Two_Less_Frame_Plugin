use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const DISPATCH_OFFSET: usize = 0x384FA10;
const DISPATCH_SIGNATURE: [u32; 4] = [0xA9BC5FF8, 0xA90157F6, 0xA9024FF4, 0xA9037BFD];
const WARMUP_PRESENTATIONS: u32 = 120;
const MAX_COMMANDS: u64 = 0x80;
const SLOT_SIZE: usize = 0x438;

static LAST_PRESENTATION: AtomicU32 = AtomicU32::new(0);
static EXTRA_COUNT: AtomicU32 = AtomicU32::new(0);
static IN_EXTRA: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = 0x384FA10)]
unsafe fn dispatch_when_ready(state: u64, mode: u32) {
    call_original!(state, mode);

    let presentation = crate::pipeline_trace::presentation_count();
    if presentation < WARMUP_PRESENTATIONS || state == 0 {
        return;
    }

    let base = state as *const u8;
    if std::ptr::read_volatile(base.add(0x1d18)) != 1
        || std::ptr::read_volatile(base.add(0x1d28)) != 1
    {
        return;
    }

    let slot = std::ptr::read_volatile(base.add(0x1d30).cast::<u64>());
    let producer_slot = std::ptr::read_volatile(base.add(0x1d20).cast::<u32>());
    if slot > 1 || producer_slot > 1 || slot != (producer_slot ^ 1) as u64 {
        return;
    }

    let batch = base.add(slot as usize * SLOT_SIZE);
    let submitted = std::ptr::read_volatile(batch.add(0x18a8).cast::<u64>());
    let produced = std::ptr::read_volatile(batch.add(0x18a0).cast::<u64>());
    if produced == 0 || produced > MAX_COMMANDS || submitted != produced {
        return;
    }

    let previous = LAST_PRESENTATION.load(Ordering::Relaxed);
    if previous == presentation
        || LAST_PRESENTATION
            .compare_exchange(
                previous,
                presentation,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        || IN_EXTRA.swap(true, Ordering::AcqRel)
    {
        return;
    }

    let attempt = EXTRA_COUNT.fetch_add(1, Ordering::Relaxed);
    if attempt == 0 {
        skyline::println!("[remove-the-frame] Flush lot pret : premiere tentative");
    }

    call_original!(state, mode);

    if attempt == 0 {
        skyline::println!("[remove-the-frame] Flush lot pret : premier retour OK");
    } else if attempt == 119 {
        skyline::println!("[remove-the-frame] Flush lot pret : 120 passages termines");
    }
    IN_EXTRA.store(false, Ordering::Release);
}

pub fn install() -> bool {
    let text = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let observed = unsafe {
        [
            std::ptr::read_volatile(text.add(DISPATCH_OFFSET / 4)),
            std::ptr::read_volatile(text.add(DISPATCH_OFFSET / 4 + 1)),
            std::ptr::read_volatile(text.add(DISPATCH_OFFSET / 4 + 2)),
            std::ptr::read_volatile(text.add(DISPATCH_OFFSET / 4 + 3)),
        ]
    };
    if observed != DISPATCH_SIGNATURE {
        skyline::println!("[remove-the-frame] Flush lot pret : signature inconnue, inactif");
        return false;
    }

    skyline::install_hook!(dispatch_when_ready);
    skyline::println!("[remove-the-frame] Flush lot pret : installe, attente 120 presentations");
    true
}
