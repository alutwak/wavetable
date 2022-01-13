use super::env::EnvStage::Done;
use super::env::{create_gate, read_gate, write_gate, Gate, ASDR};
use super::system::System;
use super::wt::{Phasor, Wavetable};
use std::sync::Arc;

type Env = ASDR;

pub struct Voice<'a> {
    // system: Arc<System>,
    osc: Phasor<'a>,
    env: Env,
    level: f32,
    pitch: f32,
    gate: Gate,
}

impl<'a> Voice<'a> {
    pub fn new(
        system: &Arc<System>,
        table: &'a Wavetable,
        att: f32,
        dec: f32,
        sus: f32,
        rel: f32,
    ) -> Self {
        let gate = create_gate(0.0);
        Voice {
            // system: system.clone(),
            osc: table.new_phasor(system),
            env: Env::new(system, att, dec, sus, rel, &gate),
            level: read_gate(&gate),
            pitch: 0.0,
            gate,
        }
    }

    pub fn note_on(&mut self, level: f32, pitch: f32) {
        self.pitch = pitch;
        write_gate(&self.gate, level);
    }

    pub fn note_off(&mut self) {
        write_gate(&self.gate, 0.0);
    }

    pub fn perform(&mut self, outbuf: &mut [f32]) -> bool {
        self.osc.perform(outbuf, self.pitch, 0.0);
        let env = self.env.perform_control();
        for out in outbuf {
            *out *= env * self.level;
        }
        self.env.stage() != Done
    }
}
