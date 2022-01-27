use super::envelope::EnvStage::*;
use super::envelope;
use super::envelope::{Gate, ASDR};
use super::system::System;
use super::wt::{Phasor, Wavetable};
use std::sync::Arc;

type Envelope = ASDR;

/** Defines a single voice within an instrument

Each note that gets played is assigned a voice for its duration. The voice manages all of the parameters of the note
and provides an interfaces for starting and releasing the note and for querying its state.
*/
pub struct Voice {
    // The oscillator
    osc: Phasor,
    // The envelope
    envelope: Envelope,
    // The overall level of the note (range of [0:1])
    level: f32,
    // The current frequency of the note (in Hz)
    pitch: f32,
    // The gate to control the envelope
    gate: Gate,
}

impl Voice {

    /** Creates a new Voice
    
    # Arguments
    * `system`: The System parameters
    * `table`:  The wavetable that the voice will use
    * `att`:    The starting attack value (in seconds)
    * `dec`:    The starting decay value (in seconds)
    * `sus`:    The starting sustain value
    * `rel`:    The starting release value (in seconds)
    */
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

    /** Start the attack stage of a note
    
    # Arguments
    * `level`: The new note's level
    * `pitch`: The new note's pitch (in Hz)
    */
    pub fn note_on(&mut self, level: f32, pitch: f32) {
        self.pitch = pitch;
        self.level = level;
        self.osc.zero();
        envelope::write_gate(&self.gate, level);
    }

    /** Starts the release stage of the note
    */
    pub fn note_off(&mut self) {
        envelope::write_gate(&self.gate, 0.0);
    }

    /** Calculates the next set of output samples and returns them in the given buffer
    
    # Arguments:
    * `outbuf`: The buffer in which to return the calculated samples
    */
    pub fn perform(&mut self, outbuf: &mut [f32]) {
        self.osc.perform(outbuf, self.pitch, 0.0);
        let envelope = self.envelope.perform_control();
        for out in outbuf {
            *out *= envelope * self.level;
        }
    }

    /** Returns whether the voice is currently active
    
    A return value of true means that the voice is active.
    */
    pub fn active(&mut self) -> bool {
        envelope::read_gate(&self.gate) > 0.0 || self.envelope.stage() != Done
    }

    /** Returns the current pitch of the voice
    */
    pub fn pitch(&mut self) -> f32 {
        self.pitch
    }
}
