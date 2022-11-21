use super::midi;
use super::midi::Message;
use std::sync::Arc;
use wavetable::system::System;
use wavetable::voice::Voice;
use wavetable::wt::Wavetable;

pub struct Instrument {
    //table: Wavetable,
    voices: Vec<Voice>,
    buffer: Vec<f32>,
}

impl Instrument {
    pub fn new(
        system: &Arc<System>,
        table: &Arc<Wavetable>,
        nvoices: usize,
        att: f32,
        dec: f32,
        sus: f32,
        rel: f32,
    ) -> Self {
        let mut inst = Instrument {
            //table,
            voices: Vec::new(),
            buffer: vec![0f32; system.bufsize()],
        };

        for _ in 0..nvoices {
            inst.voices
                .push(Voice::new(system, table, att, dec, sus, rel))
        }
        inst
    }

    pub fn perform(&mut self, outbuf: &mut [f32]) {
        for out in outbuf.iter_mut() {
            *out = 0.0;
        }
        for voice in self.voices.iter_mut() {
            if voice.active() {
                voice.perform(&mut self.buffer);
                for (i, out) in outbuf.iter_mut().enumerate() {
                    *out += self.buffer[i];
                }
            }
        }
    }

    /**
     * Right now, this just ignores the note if there are no inactive notes. In the future, we'll want to keep track of the
     * oldest note and write over that one.
     */
    pub fn note_on(&mut self, level: f32, pitch: f32) {
        for voice in self.voices.iter_mut() {
            if !voice.active() {
                voice.note_on(level, pitch);
                break;
            }
        }
    }

    pub fn note_off(&mut self, pitch: f32) {
        for voice in self.voices.iter_mut() {
            if voice.active() && voice.pitch() == pitch {
                voice.note_off();
            }
        }
    }

    pub fn map_midi(&mut self, msg: &Message) {
        match msg {
            Message::NoteOff {
                chan: _,
                note,
                vel: _,
            } => {
                let pitch = midi::map_note_equal(note);
                self.note_off(pitch);
            }
            Message::NoteOn { chan: _, note, vel } => {
                let pitch = midi::map_note_equal(note);
                if *vel == 0 {
                    self.note_off(pitch);
                } else {
                    let level = midi::map_velocity(vel);
                    self.note_on(level, pitch);
                }
            }
            _ => {}
        }
    }
}
