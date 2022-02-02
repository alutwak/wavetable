wavetable
=========

A wavetable engine for Rust.

wavetable library
-----------------

The **wavetable** library includes functionality for building fast wavetable-based oscillators.

wavesynth 
---------

**wavesynth** is a CLI application that constructs a bare-bones wavetable synthesizer from an audio clip. The synthesizer can be
played using a MIDI controller.

### Documentation

```sh
wavetable wavesynth 0.1.0
A MIDI-controlled wavetable synthesizer

USAGE:
    wavesynth [OPTIONS] <WAVETABLE> <ATTACK> <DECAY> <SUSTAIN> <RELEASE>

ARGS:
    <WAVETABLE>    Path to an audio file to use for a wavetable
    <ATTACK>       Envelope attack, in ms
    <DECAY>        Envelope decay, in ms
    <SUSTAIN>      Envelope sustain, in a range of [0..1]
    <RELEASE>      Envelope release, in ms

OPTIONS:
    -b, --bufsize <BUFSIZE>            The buffer size to use, in samples [default: 256]
    -h, --help                         Print help information
    -m, --midi-device <MIDI_DEVICE>    Optional MIDI device to use. If not given, then device will
                                       be queried
    -s, --samplerate <SAMPLERATE>      The playback samplerate, in Hz [default: 48000]
    -v, --voices <VOICES>              The maximum number of voices to use [default: 8]
    -V, --version                      Print version information
```

### Example

```sh
wavesynth test/voice.wav 30 500 0.8 600
```

Installation
============

Library installation
--------------------

### Prerequisites

```sh
# Using macports:
port install libsndfile

# Using homebrew:
brew install libsndfile
```

On Ubuntu/Debian:

```sh
apt-get install libsndfile-dev
```

### Installation

```sh
cargo install --libs --path <wavetable directory>
```

Installing wavesynth binaries
-----------------------------

### Prerequisites

The prerequisites for the library must also be satisfied.

On MacOS:

```sh
# Using macports:
port install portmidi

# Using homebrew:
brew install portmidi
```

On Ubuntu/Debian:

```sh
apt-get install libportmidi-dev
```

### Installation

```sh
cargo install --bins --path <wavetable directory>
```
