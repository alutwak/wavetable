use super::system::System;
use std::sync::{Arc, Mutex};
use self::EnvStage::*;

pub type Gate = Arc<Mutex<f32>>;

/** An ASDR envelope with linear stages

The envelope works on a range of [0, 1], so the peak amplitude will need to be adjusted by multiplying its output. This may
be adjusted in the future to increase efficiency.

The envelope is triggered by a gate, which is represented by an f32 Mutex. When the gate is considered to be open when its
value is >= 0.0 and otherwise it's considered to be closed. The envelope sequence begins at the gate's rising edge
(transitioning from closed to open), and it will continue through the attack, decay and sustain stages as long as the gate
remains upen. The release stage is triggered on the gate's falling edge (transitioning from open to close) and will
continue until either the envelope output reaches 0.0 or the gate opens again.
*/
pub struct ASDR {
    system: Arc<System>,

    // Length of the attack, in cps (cycles per second).
    att: u64,
    // Length of the decay, in cps.
    dec: u64,
    // Amplitude of the sustain. Should be in a range of [0, 1] for a normal envelope shape.
    sus: f32,
    // Length of the release, in cps.
    rel: u64,

    gate: Gate,
    prev_gate: f32,

    level: f32,
    slope: f32,
    counter: u64,
    stage: EnvStage,
}

impl ASDR {
    /** Creates a new ASDR envelope

    # Arguments

    * `att`: Attack time (in seconds)
    * `dec`: Decay time (in seconds)
    * `sus`: Sustain amplitude. Should be in a range of [0, 1] for a normal envelope shape.
    * `rel`: Release time (in seconds)
    * `gate`: The envelope's gate
    */
    pub fn new(system: &Arc<System>, att: f32, dec: f32, sus: f32, rel: f32, gate: &Gate) -> Self {
        let fs = system.samplerate();
        ASDR {
            system: system.clone(),
            att: (att * fs) as u64,
            dec: (dec * fs) as u64,
            sus,
            rel: (rel * fs) as u64,

            gate: gate.clone(),
            prev_gate: *gate.lock().unwrap(),

            level: 0.0,
            slope: 0.0,
            counter: 0,
            stage: Done,
        }
    }

    #[inline]
    pub fn set_att(&mut self, att: f32) {
        self.att = (att * self.system.samplerate()) as u64;
    }

    #[inline]
    pub fn set_dec(&mut self, dec: f32) {
        self.dec = (dec * self.system.samplerate()) as u64;
    }

    #[inline]
    pub fn set_sus(&mut self, sus: f32) {
        self.sus = sus;
    }

    #[inline]
    pub fn set_rel(&mut self, rel: f32) {
        self.rel = (rel * self.system.samplerate()) as u64;
    }

    #[inline]
    pub fn stage(&mut self) -> EnvStage {
        self.stage
    }

    #[inline]
    fn check_stage(&mut self) {
        let g = *self.gate.lock().unwrap();
        if g <= 0.0 && self.prev_gate > 0.0 {
            self.stage = Rel;
            self.counter = self.rel;
            self.slope = -self.level / self.rel as f32;
            self.prev_gate = g;
        } else if g > 0.0 && self.prev_gate <= 0.0 {
            self.stage = Att;
            self.counter = self.att;
            self.slope = (1.0 - self.level) / self.att as f32;
            self.prev_gate = g;
        } else if self.counter == 0 {
            match self.stage {
                Att => {
                    self.stage = Dec;
                    self.counter = self.dec;
                    self.slope = (self.sus - 1.0) / self.dec as f32;
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

    /** Performs the envelope operation.

    Calculates the next output.len() samples and multiplies the values in output by these values

    # Arguments

    * `outbuf`: A buffer for storing the output samples.
    */
    pub fn perform_audio(&mut self, outbuf: &mut [f32]) {
        for out in outbuf {
            if !(self.stage == Done || self.stage == Sus) {
                self.counter -= 1;
            }
            self.check_stage();
            self.level += self.slope;
            *out *= self.level;
        }
    }

    pub fn perform_control(&mut self) -> f32 {
        let cr_div = self.system.controlrate_div();
        if !(self.stage == Done || self.stage == Sus) {
            self.counter -= std::cmp::min(self.counter, cr_div as u64);
        }
        self.check_stage();
        self.level += self.slope * cr_div;
        self.level
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
pub enum EnvStage {
    Att,
    Dec,
    Sus,
    Rel,
    Done,
}

#[cfg(test)]
mod tests {
    use crate::system::System;
    use std::sync::Arc;

    use super::*;
    use float_cmp::approx_eq;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_create_asdr() {
        let system = Arc::new(System::new(1.0, 1, 1024));
        let gate = create_gate(0.0);
        let _asdr = ASDR::new(&system, 100.0, 100.0, 0.5, 100.0, &gate);
    }

    #[test]
    fn test_asdr_off_audio() {
        let system = Arc::new(System::new(1.0, 1, 1000));
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(&system, 100.0, 100.0, 0.5, 100.0, &gate);
        let mut buffer = [1.0; 1000];
        asdr.perform_audio(&mut buffer);
        for (i, val) in buffer.iter().enumerate() {
            assert_eq!(
                *val, 0.0,
                "index {} of output was {}, expected 0.0",
                i, *val
            );
        }
    }

    #[test]
    fn test_asdr_off_control() {
        // Samplerate == ctrlrate just makes the math easier
        let system = Arc::new(System::new(128.0, 128, 1000));
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(&system, 100.0, 100.0, 0.5, 100.0, &gate);
        for i in 0..1000 {
            let env = asdr.perform_control();
            assert_eq!(env, 0.0, "index {} of output was {}, expected 0.0", i, env);
        }
    }

    #[test]
    fn test_asdr_audio() {
        let system = Arc::new(System::new(1.0, 1, 1000));
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(&system, 128.0, 128.0, 0.5, 128.0, &gate);
        let mut buffer = [1.0; 1000];

        // Open the gate
        open_gate(&gate);

        // Test attack, decay and sustain
        asdr.perform_audio(&mut buffer);
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

        // Clear the buffer
        for out in buffer.iter_mut() {
            *out = 1.0;
        }

        // Close the gate
        close_gate(&gate);

        // Test release
        asdr.perform_audio(&mut buffer);
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

    #[test]
    fn test_asdr_control() {
        let system = Arc::new(System::new(128.0, 128, 1000));
        let gate = create_gate(0.0);
        let mut asdr = ASDR::new(&system, 128.0, 128.0, 0.5, 128.0, &gate);

        // Open the gate
        open_gate(&gate);

        // Test attack, decay and sustain
        //asdr.perform_audio(&mut buffer);
        let mut expected = 0.0f32;
        for i in 0..1000 {
            let val = asdr.perform_control();
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
                approx_eq!(f32, val, expected, epsilon = 1e-3),
                "ADS: index {} of output was {}, expected {}",
                i,
                val,
                expected
            );
        }

        // Close the gate
        close_gate(&gate);

        // Test release
        for i in 0..1000 {
            let val = asdr.perform_control();
            expected = if i < 128 {
                let step = -0.5 / 128.0;
                expected + step
            } else {
                0.0
            };
            assert!(
                approx_eq!(f32, val, expected, epsilon = 1e-3),
                "Release: index {} of output was {}, expected {}",
                i,
                val,
                expected
            );
        }
    }

    #[test]
    fn test_asdr_thread_audio() {
        let system = Arc::new(System::new(1.0, 1, 128));
        let gate = create_gate(0.0);
        let reader_gate = Arc::clone(&gate);
        let mut asdr = ASDR::new(&system, 128.0, 128.0, 0.5, 128.0, &gate);

        let read_thread = thread::spawn(move || {
            let mut buffer = [1.0; 128];

            // Assert that the output stays at 0 until gate is opened
            println!("Reader: waiting for gate to open");
            while read_gate(&reader_gate) == 0.0 {
                // Clear the buffer
                for out in buffer.iter_mut() {
                    *out = 1.0;
                }

                asdr.perform_audio(&mut buffer);
                for v in buffer.iter() {
                    assert!(
                        *v == 0.0 || read_gate(&reader_gate) > 0.0,
                        "Output rose above 0 before gate was opened"
                    );
                }
            }

            // Assert that output doesn't return to 0 until gate is closed
            println!("Reader: gate opened! Waiting for gate to close.");
            while read_gate(&reader_gate) > 0.0 {
                // Clear the buffer
                for out in buffer.iter_mut() {
                    *out = 1.0;
                }

                asdr.perform_audio(&mut buffer);
                for v in buffer.iter() {
                    assert!(
                        *v > 0.0 || read_gate(&reader_gate) <= 0.0,
                        "Output dropped below 0 before gate was closed"
                    );
                }
            }

            // Clear the buffer
            for out in buffer.iter_mut() {
                *out = 1.0;
            }

            println!("Reader: gate closed! Checking release behavior.");
            // Read 128 samples, which is the length of the release
            asdr.perform_audio(&mut buffer);
            assert_eq!(
                buffer[buffer.len() - 1],
                0.0,
                "The output did not make it to 0.0 within the release time"
            );
        });

        // Open gate after 1 second
        thread::sleep(Duration::from_secs(1));
        println!("Opening gate!");
        open_gate(&gate);

        // Close gate after another second
        thread::sleep(Duration::from_secs(1));
        println!("Closing gate!");
        close_gate(&gate);

        read_thread.join().unwrap();
    }
}
