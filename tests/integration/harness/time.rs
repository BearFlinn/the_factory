use bevy::prelude::*;

pub fn tick(app: &mut App) {
    app.update();
}

pub fn tick_seconds(app: &mut App, secs: f32) {
    let frames = (f64::from(secs) * 60.0).ceil() as u32;
    for _ in 0..frames {
        app.update();
    }
}

pub fn tick_n(app: &mut App, n: u32) {
    for _ in 0..n {
        app.update();
    }
}

pub fn tick_until(
    app: &mut App,
    max_frames: u32,
    condition: impl Fn(&World) -> bool,
    msg: &str,
) -> u32 {
    for frame in 0..max_frames {
        if condition(app.world()) {
            return frame;
        }
        app.update();
    }
    if condition(app.world()) {
        return max_frames;
    }
    panic!("condition not met after {max_frames} frames: {msg}");
}
