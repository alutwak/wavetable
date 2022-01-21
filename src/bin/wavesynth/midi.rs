use std::io;
use std::fmt;
use portmidi::{PortMidi, InputPort, Direction, MidiMessage};

const MIDIBUFSIZE: usize = 16;

pub enum MidiError {
    DevNotFound,
    Cancelled,
    PortMidiError(String)
}

const NOTE_OFF: u8 = 0x80;
const NOTE_ON: u8 = 0x90;
const POLY_PRESSURE: u8 = 0xA0;
const CONTROL_CHANGE: u8 = 0xB0;
const PROGRAM_CHANGE: u8 = 0xC0;
const CHANNEL_PRESSURE: u8 = 0xD0;
const PITCH_BEND: u8 = 0xE0;

pub enum Message {
    NoteOff {chan: u8, note: u8, vel: u8},
    NoteOn {chan: u8, note: u8, vel: u8},
    PolyPressure {chan: u8, note: u8, vel: u8},
    ControlChange {chan: u8, ctrl: u8, val: u8},
    ProgramChange {chan: u8, prog: u8},
    ChannelPressure {chan: u8, vel: u8},
    PitchBend {chan: u8, lsb: u8, msb: u8},
    Undefined(u8),
}

impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Error Occurred, Please Try Again!") // user-facing output
    }
}

impl fmt::Debug for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!()) // programmer-facing output
    }
}

pub fn get_midi_device(pm: &PortMidi) -> Result<InputPort, MidiError> {

    println!("Select MIDI input device number from the following options (or \"q\" to cancel):");

    let devices = pm.devices().map_err(
        |_| {
            MidiError::PortMidiError("Failed to get MIDI devices".to_string())
        })?;

    let mut valid = Vec::new();
    for (i, device) in devices.iter().enumerate() {
        if device.direction() == Direction::Input {
            println!("{}: {}", i, device.name());
            valid.push(i);
        }
    }

    if valid.is_empty() {
        println!("No input midi devices found.");
        return Err(MidiError::DevNotFound);
    }

    let index: usize;
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().eq("q") {
            println!("Never mind!");
            return Err(MidiError::Cancelled);
        }

        match input.trim().parse::<i32>() {
            Ok(i) => {
                match valid.binary_search(&(i as usize)) {
                    Ok(_) => {
                        index = i as usize;
                        break;
                    }
                    Err(_) => {
                        println!("{} is not a valid input device index", i);
                    }
                }
            },
            Err(_) => {
                println!("{} is not a valid choice", input);
            }
        }
    }

    let info = devices[index].clone();

    println!("Using {}", info.name());

    let device = InputPort::new(pm, info, MIDIBUFSIZE).map_err(
        |e| {
            MidiError::PortMidiError(format!("Failed to open MIDI port: {}", e))
        }
    )?;
    Result::Ok(device)
}

pub fn map_note(msg: &MidiMessage) -> Message {
    let status = msg.status & 0xF0;
    let chan = msg.status & 0xf;
    match status {
        NOTE_OFF => Message::NoteOff {chan, note: msg.data1, vel: msg.data2},
        NOTE_ON => Message::NoteOn {chan, note: msg.data1, vel: msg.data2},
        POLY_PRESSURE => Message::PolyPressure {chan, note: msg.data1, vel: msg.data2},
        CONTROL_CHANGE => Message::ControlChange {chan, ctrl: msg.data1, val: msg.data2},
        PROGRAM_CHANGE => Message::ProgramChange {chan, prog: msg.data1},
        CHANNEL_PRESSURE => Message::ChannelPressure {chan, vel: msg.data1},
        PITCH_BEND => Message::PitchBend {chan, lsb: msg.data1, msb: msg.data2},
        _ => Message::Undefined(status)
    }
}
