pub struct System {
    pub samplerate: f32,
    pub controlrate_div: f32,
}

impl System {
    pub fn new(samplerate: f32, controlrate_div: u64) -> Self {
        System {
            samplerate,
            controlrate_div: controlrate_div as f32,
        }
    }

    pub fn samplerate(&self) -> f32 {
        self.samplerate
    }

    pub fn controlrate_div(&self) -> f32 {
        self.controlrate_div
    }
}
