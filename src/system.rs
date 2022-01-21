pub struct System {
    samplerate: f32,
    controlrate_div: f32,
    bufsize: usize,
}

impl System {
    pub fn new(samplerate: f32, controlrate_div: u64, bufsize: usize) -> Self {
        System {
            samplerate,
            controlrate_div: controlrate_div as f32,
            bufsize
        }
    }

    pub fn samplerate(&self) -> f32 {
        self.samplerate
    }

    pub fn controlrate_div(&self) -> f32 {
        self.controlrate_div
    }

    pub fn bufsize(&self) -> usize {
        self.bufsize
    }
}
