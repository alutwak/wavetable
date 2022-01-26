use std::io;
use std::fmt;
use portmidi::{PortMidi, InputPort, Direction, MidiMessage};

const MIDIBUFSIZE: usize = 16;

static EQUAL_TEMP_MAP: [f32; 128] = [
    16.351598, 17.323914, 18.354048, 19.445436, 20.601722, 21.826764, 23.124651, 24.499715, 25.956544,
    27.5,      29.135235, 30.867706, 32.703196, 34.647829, 36.708096, 38.890873, 41.203445, 43.65353,
    46.249303, 48.999429, 51.913087, 55.,       58.27047,  61.735413, 65.40639,  69.295658, 73.41619,
    77.781746, 82.40689,  87.30706,  92.498606, 97.998859, 103.82617, 110.,      116.54094, 123.470825,
    130.81279, 138.59131, 146.83238, 155.56349, 164.81378, 174.61412, 184.99721, 195.99772, 207.65235,
    220.,      233.08188, 246.94165, 261.62557, 277.18262, 293.66476, 311.12698, 329.62756, 349.22824,
    369.99442, 391.99544, 415.30467, 440.,      466.16376, 493.8833,  523.25116, 554.36523, 587.3295,
    622.25397, 659.2551,  698.4565,  739.98885, 783.99084, 830.6094,  880.,      932.3275,  987.7666,
    1046.5023, 1108.7305, 1174.659,  1244.5079, 1318.5103, 1396.913,  1479.9777, 1567.9817, 1661.2188, 
    1760.,     1864.655,  1975.5332, 2093.0045, 2217.461,  2349.318,  2489.0159, 2637.0205, 2793.826,
    2959.9554, 3135.9635, 3322.4376, 3520.,     3729.31,   3951.0664, 4186.0093, 4434.922,  4698.636,
    4978.0317, 5274.041,  5587.652,  5919.9108, 6271.927,  6644.875,  7040.,     7458.62,   7902.133, 
    8372.019,  8869.844,  9397.273,  9956.063,  10548.082, 11175.304, 11839.821, 12543.854, 13289.75,
    14080.,    14917.24,  15804.266, 16744.036, 17739.688, 18794.545, 19912.127, 21096.164, 22350.607,
    23679.643, 25087.708
];

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

pub fn map_message(msg: &MidiMessage) -> Message {
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

pub fn map_velocity(velocity: &u8) -> f32 {
    1.0 - f32::exp( (127.0 - *velocity as f32) / 64.0 ) / 8.0
}

pub fn map_note_equal(note: &u8) -> f32 {
    EQUAL_TEMP_MAP[*note as usize]
}
