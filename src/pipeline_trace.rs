use std::sync::atomic::{AtomicU32, Ordering};

const SCENE_OFFSET: usize = 0x3725030;
const SCENE_EXPECTED: [u32; 2] = [0xD10243FF, 0xA9036FFC];

static RENDER_CALLS: AtomicU32 = AtomicU32::new(0);
static SCENE_CALLS: AtomicU32 = AtomicU32::new(0);
static PRESENT_CALLS: AtomicU32 = AtomicU32::new(0);

pub fn presentation_count() -> u32 {
    PRESENT_CALLS.load(Ordering::Acquire)
}

static LAST_RENDER: AtomicU32 = AtomicU32::new(0);
static LAST_SCENE: AtomicU32 = AtomicU32::new(0);

static RENDER_0: AtomicU32 = AtomicU32::new(0);
static RENDER_1: AtomicU32 = AtomicU32::new(0);
static RENDER_2_PLUS: AtomicU32 = AtomicU32::new(0);
static SCENE_0: AtomicU32 = AtomicU32::new(0);
static SCENE_1: AtomicU32 = AtomicU32::new(0);
static SCENE_2_PLUS: AtomicU32 = AtomicU32::new(0);

#[skyline::hook(offset = 0x3725030, inline)]
unsafe fn observe_scene(_: &skyline::hooks::InlineCtx) {
    SCENE_CALLS.fetch_add(1, Ordering::Relaxed);
}

pub fn on_render() {
    RENDER_CALLS.fetch_add(1, Ordering::Relaxed);
}

fn record(amount: u32, zero: &AtomicU32, one: &AtomicU32, many: &AtomicU32) {
    match amount {
        0 => { zero.fetch_add(1, Ordering::Relaxed); }
        1 => { one.fetch_add(1, Ordering::Relaxed); }
        _ => { many.fetch_add(1, Ordering::Relaxed); }
    }
}

pub fn on_present() {
    let n = PRESENT_CALLS.fetch_add(1, Ordering::Relaxed);
    if n >= 120 {
        return;
    }

    let renders = RENDER_CALLS.load(Ordering::Relaxed);
    let scenes = SCENE_CALLS.load(Ordering::Relaxed);
    let last_render = LAST_RENDER.swap(renders, Ordering::Relaxed);
    let last_scene = LAST_SCENE.swap(scenes, Ordering::Relaxed);

    record(renders.wrapping_sub(last_render), &RENDER_0, &RENDER_1, &RENDER_2_PLUS);
    record(scenes.wrapping_sub(last_scene), &SCENE_0, &SCENE_1, &SCENE_2_PLUS);

    if n == 119 {
        skyline::println!(
            "[remove-the-frame] Ordre CPU : 120 presentations ; appels rendu 0/1/2+=[{},{},{}] ; scene 0/1/2+=[{},{},{}]",
            RENDER_0.load(Ordering::Relaxed),
            RENDER_1.load(Ordering::Relaxed),
            RENDER_2_PLUS.load(Ordering::Relaxed),
            SCENE_0.load(Ordering::Relaxed),
            SCENE_1.load(Ordering::Relaxed),
            SCENE_2_PLUS.load(Ordering::Relaxed),
        );
    }
}

pub fn install() {
    let base = unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as *const u32
    };
    let observed = unsafe {
        [
            std::ptr::read_volatile(base.add(SCENE_OFFSET / 4)),
            std::ptr::read_volatile(base.add(SCENE_OFFSET / 4 + 1)),
        ]
    };
    if observed != SCENE_EXPECTED {
        skyline::println!("[remove-the-frame] Scene : signature inconnue, sonde inactive");
        return;
    }

    skyline::install_hook!(observe_scene);
    skyline::println!("[remove-the-frame] Sonde de sequence CPU installee (timing inchange)");
}
