use std::sync::Arc;
use std::sync::mpsc::channel;

use portmidi::PortMidi;
use clap::Parser;

mod midi;
mod instrument;
mod stream;
use midi::{MidiError, Message};
use instrument::Instrument;
use wavetable::system::System;
use wavetable::wt::Wavetable;

fn main() -> Result<(), i32> {

    let args = Args::parse();

    if let Some(midi_device) = args.midi_device.as_deref() {
        println!("MIDI Device: {}", midi_device);
    }

    let system = Arc::new(System::new(args.samplerate as f32, args.bufsize as u64, args.bufsize));

    let table = Wavetable::from_sndfile(&args.wavetable).map_err(
        |e| {
            println!("{}", e);
            1
        })?;

    let table = Arc::new(table);

    let mut instrument =  Instrument::new(
        &system,
        &table,
        args.voices,
        args.attack/1000.0,
        args.decay/1000.0,
        args.sustain,
        args.release/1000.0);

    // Create Midi Device
    let pm = PortMidi::new().unwrap();
    let midi_dev_result = midi::get_midi_device(&pm);
    if let Err(err) = midi_dev_result {
        match err {
            MidiError::Cancelled => {return Ok(());}
            MidiError::DevNotFound => {return Err(1);}
            MidiError::PortMidiError(reason) => {
                println!("Encountered portmidi error: {}", reason);
                return Err(1);
            }
        }
    }
    let mut mididev = midi_dev_result.unwrap();

    let (tx, rx) = channel::<Message>();

    let perform = move |outbuf: &mut [f32], _: &cpal::OutputCallbackInfo| {
        instrument.perform(outbuf);

        for msg in rx.try_iter() {
            instrument.map_midi(&msg);
        }
    };

    let _stream = stream::make_stream(&system, perform).map_err(
        |e| {
            eprintln!("Failed to create output stream: {}", e);
            1
        }
    )?;

    loop {
        if let Some(event) = mididev.read().unwrap() {
            let msg = midi::map_message(&event.message);
            match msg {
                Message::NoteOff {chan, note, vel} => {
                    println!("NoteOff: chan({}), note({}), vel({})", chan, note, vel);
                }
                Message::NoteOn {chan, note, vel} => {
                    println!("NoteOn: chan({}), note({}), vel({})", chan, note, vel);
                }
                _ => continue
            }
            tx.send(msg).unwrap();
        }
    }
}

#[derive(Parser)]
#[clap(version = "wavesynth 0.1.0", long_about = None)]
#[clap(about = "A MIDI-controlled wavetable synthesizer")]
struct Args {

    /// Path to an audio file to use for a wavetable
    wavetable: String,

    /// Envelope attack, in ms
    attack: f32,

    /// Envelope decay, in ms
    decay: f32,

    /// Envelope sustain, in a range of [0..1]
    sustain: f32,

    /// Envelope release, in ms
    release: f32,

    /// Optional MIDI device to use. If not given, then device will be queried
    #[clap(short, long)]
    midi_device: Option<String>,

    /// The playback samplerate, in Hz
    #[clap(short, long, default_value = "48000")]
    samplerate: usize,

    /// The buffer size to use, in samples
    #[clap(short, long, default_value = "256")]
    bufsize: usize,

    /// The maximum number of voices to use
    #[clap(short, long, default_value = "8")]
    voices: usize,
}
