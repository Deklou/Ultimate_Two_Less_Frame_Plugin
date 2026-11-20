use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const HID_OFFSET: usize = 0x3667210;
const REFRESH_OFFSET: usize = 0x3667C90;
const SNAPSHOT_OFFSET: usize = 0x3762EC0;
const STATUS_REQUEST_OFFSET: usize = 0x20876C0;
const PLAYER1_SLOT_OFFSET: usize = 0x5339860;

const HID_EXPECTED: [u32; 2] = [0xD10183FF, 0xA9044FF4];
const REFRESH_EXPECTED: [u32; 2] = [0xFC1A0FE8, 0xA90167FA];
const SNAPSHOT_EXPECTED: [u32; 2] = [0xD10203FF, 0xF90013FB];
const STATUS_REQUEST_EXPECTED: [u32; 2] = [0xF9402000, 0xF9400008];

const WINDOW: u32 = 120;
const MAX_WINDOWS: u32 = 32;

static ACTIVE: AtomicBool = AtomicBool::new(true);
static PLAYER1_SLOT: AtomicUsize = AtomicUsize::new(0);
static BUTTONS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static LAST_BUTTONS: AtomicU32 = AtomicU32::new(0);

static PRESENTS: AtomicU32 = AtomicU32::new(0);
static HID: AtomicU32 = AtomicU32::new(0);
static REFRESH: AtomicU32 = AtomicU32::new(0);
static SNAPSHOT: AtomicU32 = AtomicU32::new(0);
static VALID_BUTTON_SAMPLES: AtomicU32 = AtomicU32::new(0);
static PRESS_COUNT: AtomicU32 = AtomicU32::new(0);
static RELEASE_COUNT: AtomicU32 = AtomicU32::new(0);
static STATUS_REQUEST_COUNT: AtomicU32 = AtomicU32::new(0);

// Format : bit de validite | numero de presentation (15 bits) | numero de rendu (16 bits) | masque (32 bits).
// Il s'agit de compteurs CPU, pas d'identifiants de frames affichees par le GPU.
static FIRST_PRESS: AtomicU64 = AtomicU64::new(0);
static LAST_PRESS: AtomicU64 = AtomicU64::new(0);
static FIRST_STATUS: AtomicU64 = AtomicU64::new(0);
static LAST_STATUS: AtomicU64 = AtomicU64::new(0);

fn event(value: u32) -> u64 {
    let present = PRESENTS.load(Ordering::Relaxed) as u64;
    let render = crate::render_probe::snapshot().0 as u64;
    (1u64 << 63) | ((present & 0x7fff) << 48) | ((render & 0xffff) << 32) | value as u64
}

fn record_event(first: &AtomicU64, last: &AtomicU64, value: u32) {
    let encoded = event(value);
    let _ = first.compare_exchange(0, encoded, Ordering::Relaxed, Ordering::Relaxed);
    last.store(encoded, Ordering::Relaxed);
}

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
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    SNAPSHOT.fetch_add(1, Ordering::Relaxed);

    // FUN_7103762EC0 lit le premier slot a 0x7105339860 ; le pointeur de l'etat
    // du controleur se trouve a l'offset +0x10, son masque de boutons a +0x08.
    let slot = PLAYER1_SLOT.load(Ordering::Relaxed) as *const *const u8;
    if slot.is_null() {
        return;
    }
    let controller = std::ptr::read_volatile(slot);
    if controller.is_null() {
        BUTTONS_INITIALIZED.store(false, Ordering::Relaxed);
        return;
    }
    let state = std::ptr::read_volatile(controller.add(0x10).cast::<*const u8>());
    if state.is_null() || std::ptr::read_volatile(state.add(0xB8)) == 0 {
        BUTTONS_INITIALIZED.store(false, Ordering::Relaxed);
        return;
    }

    let held = std::ptr::read_volatile(state.add(0x08).cast::<u32>());
    VALID_BUTTON_SAMPLES.fetch_add(1, Ordering::Relaxed);

    if !BUTTONS_INITIALIZED.swap(true, Ordering::Relaxed) {
        LAST_BUTTONS.store(held, Ordering::Relaxed);
        return;
    }

    let previous = LAST_BUTTONS.swap(held, Ordering::Relaxed);
    let pressed = held & !previous;
    let released = previous & !held;
    if pressed != 0 {
        PRESS_COUNT.fetch_add(1, Ordering::Relaxed);
        record_event(&FIRST_PRESS, &LAST_PRESS, pressed);
    }
    if released != 0 {
        RELEASE_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

// Hypothese a verifier : ce wrapper est appele par les scripts des combattants.
// Il ne prouve ni qu'un statut a ete accepte, ni quel joueur l'a demande.
#[skyline::hook(offset = 0x20876C0, inline)]
unsafe fn observe_status_request(ctx: &skyline::hooks::InlineCtx) {
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    STATUS_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    record_event(&FIRST_STATUS, &LAST_STATUS, ctx.registers[1].w());
}

// Appele par la sonde passive de presentation deja validee sur Ryujinx.
pub fn on_present() {
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    let n = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if n % WINDOW != 0 {
        return;
    }

    let hid = HID.swap(0, Ordering::Relaxed);
    let refresh = REFRESH.swap(0, Ordering::Relaxed);
    let snapshots = SNAPSHOT.swap(0, Ordering::Relaxed);
    let valid = VALID_BUTTON_SAMPLES.swap(0, Ordering::Relaxed);
    let presses = PRESS_COUNT.swap(0, Ordering::Relaxed);
    let releases = RELEASE_COUNT.swap(0, Ordering::Relaxed);
    let statuses = STATUS_REQUEST_COUNT.swap(0, Ordering::Relaxed);
    let first_press = FIRST_PRESS.swap(0, Ordering::Relaxed);
    let last_press = LAST_PRESS.swap(0, Ordering::Relaxed);
    let first_status = FIRST_STATUS.swap(0, Ordering::Relaxed);
    let last_status = LAST_STATUS.swap(0, Ordering::Relaxed);

    skyline::println!(
        "[remove-the-frame] Input diag {}/{} : HID={} maj={} copies={} boutons_valides={} appuis={} relaches={} demandes_statut={}",
        n / WINDOW, MAX_WINDOWS, hid, refresh, snapshots, valid, presses, releases, statuses
    );

    if first_press != 0 {
        skyline::println!(
            "[remove-the-frame] Boutons joueur1 : premier masque=0x{:08X} apres Present#{} Rendu#{} ; dernier masque=0x{:08X} apres Present#{} Rendu#{}",
            first_press as u32,
            (first_press >> 48) & 0x7fff,
            (first_press >> 32) & 0xffff,
            last_press as u32,
            (last_press >> 48) & 0x7fff,
            (last_press >> 32) & 0xffff
        );
    }
    if first_status != 0 {
        skyline::println!(
            "[remove-the-frame] Statuts candidats : premier={} apres Present#{} Rendu#{} ; dernier={} apres Present#{} Rendu#{}",
            first_status as u32,
            (first_status >> 48) & 0x7fff,
            (first_status >> 32) & 0xffff,
            last_status as u32,
            (last_status >> 48) & 0x7fff,
            (last_status >> 32) & 0xffff
        );
    }

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
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize
    };
    PLAYER1_SLOT.store(base + PLAYER1_SLOT_OFFSET, Ordering::Relaxed);

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
        skyline::println!("[remove-the-frame] Input diag : masque de boutons observe");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature copie inconnue");
    }

    if signature_ok(STATUS_REQUEST_OFFSET, STATUS_REQUEST_EXPECTED) {
        skyline::install_hook!(observe_status_request);
        skyline::println!("[remove-the-frame] Input diag : demandes de statut observees");
    } else {
        skyline::println!("[remove-the-frame] Input diag : signature statut inconnue");
    }
}
