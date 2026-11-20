const INDEX_OFFSET: usize = 0x386B0FC;
const VANILLA: u32 = 0x11000929; // add w9, w9, #2
const NO_SHIFT: u32 = 0xD503201F; // nop : w9 conserve l'index brut

pub fn install() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let expected = [0xB9403109, VANILLA, 0x5295556A, 0x72B5554A];
    let offset = INDEX_OFFSET / 4;
    let actual = unsafe {
        [
            std::ptr::read_volatile(base.add(offset - 1)),
            std::ptr::read_volatile(base.add(offset)),
            std::ptr::read_volatile(base.add(offset + 1)),
            std::ptr::read_volatile(base.add(offset + 2)),
        ]
    };
    if actual != expected {
        skyline::println!("[remove-the-frame] Index +0 : signature inconnue, aucun patch");
        return;
    }

    if skyline::patching::Patch::in_text(INDEX_OFFSET)
        .data(NO_SHIFT)
        .is_err()
    {
        skyline::println!("[remove-the-frame] Index +0 : echec du patch, mode vanilla");
        return;
    }

    let written = unsafe { std::ptr::read_volatile(base.add(offset)) };
    if written == NO_SHIFT {
        skyline::println!("[remove-the-frame] Index +0 : instruction remplacee (VSync inchangee)");
    } else {
        skyline::println!("[remove-the-frame] Index +0 : lecture de controle inattendue");
    }
}
