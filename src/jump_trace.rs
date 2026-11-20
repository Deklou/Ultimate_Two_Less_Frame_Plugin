use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

// Aucune modification d'etat de jeu : on observe l'entree dans la fonction.
// La validation de l'identite NRO et des prologues est faite dans nro_probe.
const WINDOW: u32 = 120;
const MAX_PRESENTS: u32 = 3840;

static PRESENTS: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);

static COUNT: AtomicU32 = AtomicU32::new(0);
static FIRST: AtomicU64 = AtomicU64::new(0);
static LAST: AtomicU64 = AtomicU64::new(0);
static FIRST_CONTEXT: AtomicUsize = AtomicUsize::new(0);
static LAST_CONTEXT: AtomicUsize = AtomicUsize::new(0);

// Enregistre des compteurs CPU ; aucune information sur la frame affichee.
unsafe extern "C" fn observe_jump_squat(ctx: &skyline::hooks::InlineCtx) {
    if !ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    let present = PRESENTS.load(Ordering::Relaxed) as u64;
    let render = crate::render_probe::snapshot().0 as u64;
    let stamp = (present << 32) | (render & 0xffff_ffff);
    let context = ctx.registers[0].x() as usize;

    COUNT.fetch_add(1, Ordering::Relaxed);
    if FIRST.compare_exchange(0, stamp, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        FIRST_CONTEXT.store(context, Ordering::Relaxed);
    }
    LAST.store(stamp, Ordering::Relaxed);
    LAST_CONTEXT.store(context, Ordering::Relaxed);
}

pub fn install(address: usize) {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    skyline::println!(
        "[remove-the-frame] JumpSquat : tentative d'installation du hook NRO a 0x{:X}",
        address
    );

    // Cette fonction ne donne pas de statut de succes. Verifier le log
    // And64InlineHook et les appels observes lors du premier test en jeu.
    unsafe {
        skyline::hooks::A64InlineHook(
            address as *const skyline::libc::c_void,
            observe_jump_squat as *const skyline::libc::c_void,
        );
    }
    ACTIVE.store(true, Ordering::Relaxed);
}

pub fn on_present() {
    let n = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if !ACTIVE.load(Ordering::Relaxed) || n % WINDOW != 0 {
        return;
    }

    let count = COUNT.swap(0, Ordering::Relaxed);
    let first = FIRST.swap(0, Ordering::Relaxed);
    let last = LAST.swap(0, Ordering::Relaxed);
    let first_context = FIRST_CONTEXT.swap(0, Ordering::Relaxed);
    let last_context = LAST_CONTEXT.swap(0, Ordering::Relaxed);

    skyline::println!(
        "[remove-the-frame] JumpSquat diag : apres Present#{} ; entrees={}",
        n, count
    );
    if first != 0 {
        skyline::println!(
            "[remove-the-frame] JumpSquat : premier apres Present#{} Rendu#{} contexte=0x{:X} ; dernier apres Present#{} Rendu#{} contexte=0x{:X}",
            first >> 32,
            first as u32,
            first_context,
            last >> 32,
            last as u32,
            last_context
        );
    }

    if n >= MAX_PRESENTS {
        ACTIVE.store(false, Ordering::Relaxed);
        skyline::println!("[remove-the-frame] JumpSquat diag : collecte terminee");
    }
}
