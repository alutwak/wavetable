use clap::Parser;
use portmidi::PortMidi;


mod midi;
mod instrument;
use midi::{MidiError, Message};
use instrument::Instrument;
use wavetable::wt::Wavetable;

fn main() -> Result<(), i32> {

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

    loop {
        if let Some(event) = mididev.read().unwrap() {
            match midi::map_note(&event.message) {
                Message::NoteOff {chan, note, vel} => {
                    println!("NoteOff[{}]: chan({}), note({}), vel({})", event.timestamp, chan, note, vel);
                }
                Message::NoteOn {chan, note, vel} => {
                    println!("NoteOn[{}]: chan({}), note({}), vel({})", event.timestamp, chan, note, vel);
                }
                Message::PolyPressure {chan, note, vel} => {
                    println!("PolyPressure[{}]: chan({}), note({}), vel({})", event.timestamp, chan, note, vel);
                }
                Message::ControlChange {chan, ctrl, val} => {
                    println!("ControlChange[{}]: chan({}), ctrl({}), vel({})", event.timestamp, chan, ctrl, val);
                }
                Message::ProgramChange {chan, prog} => {
                    println!("ProgramChange[{}]: chan({}), prog({})", event.timestamp, chan, prog);
                }
                Message::ChannelPressure {chan, vel} => {
                    println!("ChanPressure[{}]: chan({}), vel({})", event.timestamp, chan, vel);
                }
                Message::PitchBend {chan, lsb, msb} => {
                    println!("PitchBend[{}]: chan({}), pitch({})", event.timestamp, chan, lsb as u16 | (msb as u16) << 8);
                }
                _ => {
                    println!("Event ({}): status ({}): {}, {}, {}",
                             event.timestamp,
                             event.message.status,
                             event.message.data1,
                             event.message.data2,
                             event.message.data3);

                }
            }
        }
    }

    Ok(())
}


// struct Args {


// }
