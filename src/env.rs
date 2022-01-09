use std::sync::{Arc, Mutex};

pub type Gate = Arc<Mutex<f32>>;

pub struct ASDR {
    pub att: f32,
    pub dec: f32,
    pub sus: f32,
    pub rel: f32,

    gate: Gate,
    prev_gate: f32,

    level: f32,
    slope: f32,
    counter: u64,
    stage: EnvStage,
}

impl ASDR {
    pub fn new(att: u64, dec: u64, sus: f32, rel: u64, gate: &Gate) -> Self {
        ASDR {
            att: att as f32,
            dec: dec as f32,
            sus,
            rel: rel as f32,

            gate: Arc::clone(gate),
            prev_gate: *gate.lock().unwrap(),

            level: 0.0,
            slope: 0.0,
            counter: 0,
            stage: Done,
        }
    }

    #[inline]
    fn check_stage(&mut self) {
        let g = *self.gate.lock().unwrap();
        if g <= 0.0 && self.prev_gate > 0.0 {
            self.stage = Rel;
            self.counter = self.rel as u64;
            self.slope = -self.level / self.rel as f32;
            self.prev_gate = g;
        } else if g > 0.0 && self.prev_gate <= 0.0 {
            self.stage = Att;
            self.counter = self.att as u64;
            self.slope = 1.0 / self.att as f32;
            self.prev_gate = g;
        } else if self.counter == 0 {
            match self.stage {
                Att => {
                    self.stage = Dec;
                    self.counter = self.dec as u64;
                    self.slope = (self.sus - 1.0) / self.dec;
                }
                Dec => {
                    self.stage = Sus;
                    self.slope = 0.0;
                }
                Rel => {
                    self.stage = Done;
                    self.slope = 0.0;
                }
                _ => {}
            }
        }
    }

    pub fn perform(&mut self, outbuf: &mut [f32]) {
        for out in outbuf {
            if !(self.stage == Done || self.stage == Sus) {
                self.counter -= 1;
            }
            self.check_stage();
            self.level += self.slope;
            *out = self.level;
        }
    }
}

#[inline]
pub fn create_gate(val: f32) -> Gate {
    Arc::new(Mutex::new(val))
}

#[inline]
pub fn read_gate(gate: &Gate) -> f32 {
    *gate.lock().unwrap()
}

#[inline]
pub fn write_gate(gate: &Gate, val: f32) {
    *gate.lock().unwrap() = val;
}

#[inline]
pub fn open_gate(gate: &Gate) {
    write_gate(gate, 1.0);
}

#[inline]
pub fn close_gate(gate: &Gate) {
    write_gate(gate, 0.0);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvStage {
    Att,
    Dec,
    Sus,
    Rel,
    Done,
}

use crate::env::EnvStage::*;

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;

    #[test]
    fn test_create_asdr() {
        let gate = create_gate(0.0);
        let _asdr = ASDR::new(100, 100, 0.5, 100, &gate);
    }

    #[test]
    fn test_asdr_off() {
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(100, 100, 0.5, 100, &gate);
        let mut buffer = [0.0; 1000];
        asdr.perform(&mut buffer);
        for (i, val) in buffer.iter().enumerate() {
            assert_eq!(
                *val, 0.0,
                "index {} of output was {}, expected 0.0",
                i, *val
            );
        }
    }

    #[test]
    fn test_asdr() {
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(128, 128, 0.5, 128, &gate);
        let mut buffer = [0.0; 1000];

        // Open the gate
        open_gate(&gate);

        // Test attack, decay and sustain
        asdr.perform(&mut buffer);
        let mut expected = 0.0f32;
        for (i, val) in buffer.iter().enumerate() {
            expected = if i < 128 {
                let step = 1.0 / 128.0;
                expected + step
            } else if i < 256 {
                let step = -0.5 / 128.0;
                expected + step
            } else {
                0.5
            };
            assert!(
                approx_eq!(f32, *val, expected, epsilon = 1e-3),
                "ADS: index {} of output was {}, expected {}",
                i,
                *val,
                expected
            );
        }

        // Close the gate
        close_gate(&gate);

        // Test release
        asdr.perform(&mut buffer);
        for (i, val) in buffer.iter().enumerate() {
            expected = if i < 128 {
                let step = -0.5 / 128.0;
                expected + step
            } else {
                0.0
            };
            assert!(
                approx_eq!(f32, *val, expected, epsilon = 1e-3),
                "Release: index {} of output was {}, expected {}",
                i,
                *val,
                expected
            );
        }
    }
}
