use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// Identite du lua2cpp_common.nro extrait par l'utilisateur.
// Ce sont des offsets NRO, pas des offsets du main de Smash.
const JUMP_SQUAT_OFFSET: usize = 0x2245C0;
const JUMP_OFFSET: usize = 0x172040;

const JUMP_SQUAT_EXPECTED: [u32; 2] = [0xD10143FF, 0xA9034FF4];
const JUMP_EXPECTED: [u32; 2] = [0xD10183FF, 0xA9044FF4];
const BUILD_ID: [u32; 5] = [
    0x7A17F513, 0x15F0867E, 0xAA221A0F, 0xAC1E619E, 0x7C3910D0,
];

const JUMP_SQUAT_SYMBOL: &[u8] =
    b"_ZN7lua2cpp16L2CFighterCommon16status_JumpSquatEv\0";
const JUMP_SYMBOL: &[u8] =
    b"_ZN7lua2cpp16L2CFighterCommon11status_JumpEv\0";

const WINDOW: u32 = 120;
const MAX_CHECKS: u32 = 32;

static PRESENTS: AtomicU32 = AtomicU32::new(0);
static CHECKS: AtomicU32 = AtomicU32::new(0);
static COMPLETE: AtomicBool = AtomicBool::new(false);

fn lookup(symbol: &[u8]) -> usize {
    let mut address = 0usize;
    unsafe {
        // Cherche un symbole deja charge. N'installe aucun hook NRO.
        let _ = skyline::nn::ro::LookupSymbol(
            &mut address as *mut usize,
            symbol.as_ptr().cast(),
        );
    }
    address
}

unsafe fn instructions(address: usize) -> [u32; 2] {
    let p = address as *const u32;
    [std::ptr::read_volatile(p), std::ptr::read_volatile(p.add(1))]
}

// Diagnostic ponctuel, appele par la sonde de presentation deja en place.
pub fn on_present() {
    if COMPLETE.load(Ordering::Relaxed) {
        return;
    }
    let n = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if n % WINDOW != 0 {
        return;
    }

    let check = CHECKS.fetch_add(1, Ordering::Relaxed) + 1;
    if check > MAX_CHECKS {
        COMPLETE.store(true, Ordering::Relaxed);
        return;
    }

    let squat = lookup(JUMP_SQUAT_SYMBOL);
    let jump = lookup(JUMP_SYMBOL);
    if squat == 0 || jump == 0 {
        if check == 1 || check % 4 == 0 || check == MAX_CHECKS {
            skyline::println!(
                "[remove-the-frame] NRO saut : recherche {}/{} ; JumpSquat={} Jump={}",
                check, MAX_CHECKS, squat != 0, jump != 0
            );
        }
        if check == MAX_CHECKS {
            COMPLETE.store(true, Ordering::Relaxed);
            skyline::println!("[remove-the-frame] NRO saut : symboles indisponibles");
        }
        return;
    }

    // Les deux symboles doivent resider dans la meme image, avec les
    // offsets de l'extraction. Si ce n'est pas le cas, on ne lit pas l'en-tete.
    if squat < JUMP_SQUAT_OFFSET
        || jump < JUMP_OFFSET
        || squat - JUMP_SQUAT_OFFSET != jump - JUMP_OFFSET
    {
        skyline::println!(
            "[remove-the-frame] NRO saut : symboles trouves, offsets differents (JumpSquat=0x{:X} Jump=0x{:X})",
            squat, jump
        );
        COMPLETE.store(true, Ordering::Relaxed);
        return;
    }

    let expected_base = squat - JUMP_SQUAT_OFFSET;
    let squat_actual = unsafe { instructions(squat) };
    let jump_actual = unsafe { instructions(jump) };
    if squat_actual != JUMP_SQUAT_EXPECTED || jump_actual != JUMP_EXPECTED {
        skyline::println!(
            "[remove-the-frame] NRO saut : prologues differents (JumpSquat=[{:08X},{:08X}] Jump=[{:08X},{:08X}])",
            squat_actual[0], squat_actual[1], jump_actual[0], jump_actual[1]
        );
        COMPLETE.store(true, Ordering::Relaxed);
        return;
    }

    // L'en-tete NRO0 et le Build ID n'ont de sens qu'une fois les deux
    // symboles et leurs prologues verifies.
    let magic = unsafe {
        std::ptr::read_volatile((expected_base + 0x10) as *const u32)
    };
    if magic != 0x304F524E {
        skyline::println!(
            "[remove-the-frame] NRO saut : en-tete inattendu 0x{:08X}, base candidate 0x{:X}",
            magic, expected_base
        );
        COMPLETE.store(true, Ordering::Relaxed);
        return;
    }

    let mut matches = true;
    let mut runtime = [0u32; 5];
    for (i, word) in runtime.iter_mut().enumerate() {
        *word = unsafe {
            std::ptr::read_volatile((expected_base + 0x40 + i * 4) as *const u32)
        };
        if *word != BUILD_ID[i] {
            matches = false;
        }
    }

    skyline::println!(
        "[remove-the-frame] NRO saut : NRO0 valide ; Build ID identique={} ; base=0x{:X} ; ID=[{:08X},{:08X},{:08X},{:08X},{:08X}]",
        matches, expected_base, runtime[0], runtime[1], runtime[2], runtime[3], runtime[4]
    );
    COMPLETE.store(true, Ordering::Relaxed);
    if matches {
        // Le hook n'est installe qu'apres correspondance complete avec le NRO extrait.
        crate::jump_trace::install(squat);
        crate::jump_state_trace::install(jump);
    }
}

pub fn install() {
    skyline::println!(
        "[remove-the-frame] NRO saut : validation des symboles active, JumpSquat sous controle d'identite"
    );
}
