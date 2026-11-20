use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const HID_OFFSET: usize = 0x3667210;
const REFRESH_OFFSET: usize = 0x3667C90;
const SNAPSHOT_OFFSET: usize = 0x3762EC0;
const MAPPING_OFFSET: usize = 0x337B690;

const HID_EXPECTED: [u32; 2] = [0xD10183FF, 0xA9044FF4];
const REFRESH_EXPECTED: [u32; 2] = [0xFC1A0FE8, 0xA90167FA];
const SNAPSHOT_EXPECTED: [u32; 2] = [0xD10203FF, 0xF90013FB];
const MAPPING_EXPECTED: [u32; 2] = [0xD10283FF, 0xFD001BE8];

const WINDOW: u32 = 120;
const MAX_WINDOWS: u32 = 32;

static ACTIVE: AtomicBool = AtomicBool::new(true);
static PRESENTS: AtomicU32 = AtomicU32::new(0);
static HID: AtomicU32 = AtomicU32::new(0);
static REFRESH: AtomicU32 = AtomicU32::new(0);
static SNAPSHOT: AtomicU32 = AtomicU32::new(0);
static MAPPING: AtomicU32 = AtomicU32::new(0);
static MAPPING_SINCE_PRESENT: AtomicU32 = AtomicU32::new(0);
static MAP_0: AtomicU32 = AtomicU32::new(0);
static MAP_1: AtomicU32 = AtomicU32::new(0);
static MAP_MANY: AtomicU32 = AtomicU32::new(0);

#[skyline::hook(offset = 0x3667210, inline)]
unsafe fn observe_hid(_: &skyline::hooks::InlineCtx) {
    if ACTIVE.load(Ordering::Relaxed) {
        HID.fetch_add(1, Ordering::Relaxed);
    }
}

#[skyline::hook(offset = 0x3667C90, inline)]
unsafe fn observe_refresh(_: &skyline::hooks::InlineCtx) {
    if ACTIVE.load(Ordering::Relaxed) {
        REFRESH.fetch_add(1, Ordering::Relaxed);
    }
}

#[skyline::hook(offset = 0x3762EC0, inline)]
unsafe fn observe_snapshot(_: &skyline::hooks::InlineCtx) {
    if ACTIVE.load(Ordering::Relaxed) {
        SNAPSHOT.fetch_add(1, Ordering::Relaxed);
    }
}

#[skyline::hook(offset = 0x337B690, inline)]
unsafe fn observe_mapping(_: &skyline::hooks::InlineCtx) {
    if ACTIVE.load(Ordering::Relaxed) {
        MAPPING.fetch_add(1, Ordering::Relaxed);
        MAPPING_SINCE_PRESENT.fetch_add(1, Ordering::Relaxed);
    }
}

// Appelé par la sonde de présentation existante, sans changer les registres.
pub fn on_present() {
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    let n = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    let mappings = MAPPING_SINCE_PRESENT.swap(0, Ordering::Relaxed);
    match mappings {
        0 => { MAP_0.fetch_add(1, Ordering::Relaxed); }
        1 => { MAP_1.fetch_add(1, Ordering::Relaxed); }
        _ => { MAP_MANY.fetch_add(1, Ordering::Relaxed); }
    }

    if n % WINDOW != 0 {
        return;
    }

    let hid = HID.swap(0, Ordering::Relaxed);
    let refresh = REFRESH.swap(0, Ordering::Relaxed);
    let snapshot = SNAPSHOT.swap(0, Ordering::Relaxed);
    let mapping = MAPPING.swap(0, Ordering::Relaxed);
    let map_0 = MAP_0.swap(0, Ordering::Relaxed);
    let map_1 = MAP_1.swap(0, Ordering::Relaxed);
    let map_many = MAP_MANY.swap(0, Ordering::Relaxed);

    skyline::println!(
        "[remove-the-frame] Input diag fenetre {}/{} : HID={}, actualisation={}, copie={}, mappage={}, mappage/Present 0/1/2+=[{},{},{}]",
        n / WINDOW,
        MAX_WINDOWS,
        hid,
        refresh,
        snapshot,
        mapping,
        map_0,
        map_1,
        map_many
    );

    if n >= MAX_WINDOWS * WINDOW {
        ACTIVE.store(false, Ordering::Relaxed);
        skyline::println!("[remove-the-frame] Input diag : collecte terminee");
    }
}

fn signature_ok(offset: usize, expected: [u32; 2]) -> bool {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let actual = unsafe {
        [
            std::ptr::read_volatile(base.add(offset / 4)),
            std::ptr::read_volatile(base.add(offset / 4 + 1)),
        ]
    };
    actual == expected
}

pub fn install() {
    if signature_ok(HID_OFFSET, HID_EXPECTED) {
        skyline::install_hook!(observe_hid);
        skyline::println!("[remove-the-frame] Input diag : lecture HID observee");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature HID inconnue");
    }

    if signature_ok(REFRESH_OFFSET, REFRESH_EXPECTED) {
        skyline::install_hook!(observe_refresh);
        skyline::println!("[remove-the-frame] Input diag : actualisation observee");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature actualisation inconnue");
    }

    if signature_ok(SNAPSHOT_OFFSET, SNAPSHOT_EXPECTED) {
        skyline::install_hook!(observe_snapshot);
        skyline::println!("[remove-the-frame] Input diag : copie des controles observee");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature copie inconnue");
    }

    if signature_ok(MAPPING_OFFSET, MAPPING_EXPECTED) {
        skyline::install_hook!(observe_mapping);
        skyline::println!("[remove-the-frame] Input diag : mappage observe");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature mappage inconnue");
    }
}
