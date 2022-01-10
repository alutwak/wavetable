static mut SAMPLERATE: f32 = 0.0;

pub fn get_samplerate() -> f32 {
    unsafe { SAMPLERATE }
}

pub fn set_samplerate(fs: f32) {
    unsafe {
        SAMPLERATE = fs;
    }
}
