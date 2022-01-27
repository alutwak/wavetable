use super::envelope::EnvStage::*;
use super::envelope;
use super::envelope::{Gate, ASDR};
use super::system::System;
use super::wt::{Phasor, Wavetable};
use std::sync::Arc;

type Envelope = ASDR;

pub struct Voice {
    // system: Arc<System>,
    osc: Phasor,
    envelope: Envelope,
    level: f32,
    pitch: f32,
    gate: Gate,
}

impl Voice {
    pub fn new(
        system: &Arc<System>,
        table: &Arc<Wavetable>,
        att: f32,
        dec: f32,
        sus: f32,
        rel: f32,
    ) -> Self {
        let gate = envelope::create_gate(0.0);
        Voice {
            // system: system.clone(),
            osc: Phasor::new(system, table),
            envelope: Envelope::new(system, att, dec, sus, rel, &gate),
            level: envelope::read_gate(&gate),
            pitch: 0.0,
            gate,
        }
    }

    pub fn note_on(&mut self, level: f32, pitch: f32) {
        self.pitch = pitch;
        self.level = level;
        self.osc.zero();
        envelope::write_gate(&self.gate, level);
    }

    pub fn note_off(&mut self) {
        envelope::write_gate(&self.gate, 0.0);
    }

    pub fn perform(&mut self, outbuf: &mut [f32]) {
        self.osc.perform(outbuf, self.pitch, 0.0);
        let envelope = self.envelope.perform_control();
        for out in outbuf {
            *out *= envelope * self.level;
        }
    }

    pub fn active(&mut self) -> bool {
        envelope::read_gate(&self.gate) > 0.0 || self.envelope.stage() != Done
    }

    pub fn pitch(&mut self) -> f32 {
        self.pitch
    }
}
