use super::system::System;
use std::f32::consts::PI;
use std::num::Wrapping;
use std::sync::Arc;

/** An interpolating wavetable oscillator

Because of the requirements of the interpolation algorithm used, there are a couple of limits on the acceptable table
size:
1. it must be a power of two
2. it can be no larger than 131071 (or 2^17).

This may have some implications if you are trying to build a wavetable from a sampled waveform, but if your sample does
not satisfy these requirements (likely), you simply have to resample it so that it does. In the future, this library will
provide a function that can do this for you, but for now, you'll either need to do this programatically yourself, or
you'll have to pre-process your sample with your DAW or with some other audo processing tool, such as
[Audacity](https://www.audacityteam.org).

Note: The algorithms used for this implementation were based off of supercollider's Osc Ugen see
[here](https://github.com/supercollider/supercollider/blob/cea67fcd49eb899366d6f7252c70157c5bc8b18f/server/plugins/OscUGens.cpp#L1247)

# Examples

```
# use wavetable::wt::Wavetable;
# use wavetable::system::System;
# use std::sync::Arc;
let system = Arc::new(System::new(44100.0, 1));
// Create a wavetable that ramps from 0 to 128.
let table = Vec::from_iter((0..128).map(|v| -> f32 {v as f32}));
let wt = Wavetable::new(&table);

let mut phasor = wt.new_phasor(&system);

// Generate 1 second of a 440Hz waveform
let freq = 440.0;
let phase = 0.0;
let mut outbuf = [0.0f32; 1025];
phasor.perform(&mut outbuf, freq, phase);
```

# Linear interpolation algorithm

This wavetable uses linear interpolation to generate signals of arbitrary frequency. For a given wavetable, you can calulate
the linear interpolation a phase of n.m (which is 0.m points between index n and n+1) using the following equation:

```ignore
y(n.m) = x[n] + 0.m * (x[n + 1] - x[n])
```

This requires three obvious arithmetic operations (an addtion, a multiplication and a subtraction) and two indexing
operations. Less obviously, it also requires a calculation of n and 0.m (which are fixed-point calculations,
remember). It also requires either an extra register to store the `x[n]` value (or an additional indexing
operation). These are fairly modest requirements, but when you're performing this procedure thousands of times per second
for a single signal, and when you may have a dozen or more signals running at a time, saving just one or two of these
operations can make a real difference.

## Why use two tables?

The Wavetable class uses two tables under the hood in order to eliminate a few of these operations during
operation. Instead of directly storing the table values, each table stores a pre-calculated portion of the interpolation
calculation, saving us a subtraction, a register copy (or an index operation), and a floating point subtraction in the
calculation of 0.m (essentially, we get to calculate 1.m instead, which turns out to be cheaper). In exchange, we double
the memory footprint of the wavetable (which can't be more than 130kB extra). Here's a quick rundown of the table
equations (where `m` is the fractional part of the phase):

```ignore
tbl1 = (2 * val1) - val2
tbl2 = val2 - val1

out = tbl1 + (tbl2 * (1 + m))
    = (2 * val1) - val2 + ((val2 - val1) * (1 + m))
    = 2a - b + (b - a) + (b - a) * m
    = a + (b - a) * m
```
*/
pub struct Wavetable {
    // Stores 2 * x[n] - x[n+1]
    table1: Vec<f32>,
    // Stores x[n + 1] - x[n]
    table2: Vec<f32>,
    // Masks the valid integral index bits
    lomask: i32,
}

/** Generates a signal from a Wavetable by sweeping a phase across the table and giving the value at each phase sample.

The phase is tracked using a 32-bit, fixed-point value (signed to enable negative frequencies). Using a fixed-point value
prevents the noise and modulation that floating point precision errors would introduce. The upshot of this is that the phase
will change at a constant, predictable rate, but that the input frequencies and phase offsets (which are floating point) will
be quantized (notice how both of these are cast to i32 values in Wavetable::perform()).

# Phase calculations

The phase is defined in units of the wavetable index, such that a phase of n.0 retrieves the nth wavetable value (see the
[`Wavetable`] documenation for more about this). It's represented with a signed, fixed-point value with 16 fractional bits.

## Calculating the index from the phase

The wavetable index is the integral part of the phase value and is retrieved simply by right-shifting 16 bits. Then, we
also need to perform a modulo operation to keep the index within the valid wavetable range. Because the table length is a
power of two, we can calculate the modulo of the phase simply by masking in the valid bits. For instance, for a 16 sample
wavetable, the mask will be `0xF`, which will mask out any values greater than 15. The efficiency of doing this, as
opposed to using the more expensive `%` operator, is the reason that the table length must be a power of two.

## Calculating the phase's fractional value

The method for getting the fractional value from the fixed-point phase is (I think) super cool. I've never used
fixed-point values on a platform that supports floating point, so it's possible that this is a common technique and I've
just never seen it before, but it's extremely efficient and requires no arithmetic operations (unless you count a shift
and an `&`). This is encapsulated by the following code snippet:

```ignore
#[repr(C)]
union PhaseConv {
    iphase: i32,
    fphase: f32
}

#[inline]
fn phase_frac1(phase: i32) -> f32 {
    let p = PhaseConv {
        iphase: 0x3F800000 | (0x007FFF80 & ((phase) << 7))
    };
    unsafe {
        p.fphase
    }
}
```

The `phase_frac1()` function essentially constructs a floating point value in which the phase's fractional bits make up
the significand, the exponent is 0, and the sign is positive. This creates a binary value that, as a floating point
representation, equals `1.m`, `m` being the phase's fractional component. If we just wanted the pure fractional part then
we would have to subtract the 1.0 from the value, but because of the way that the two tables were pre-calculated, this
value can simply be multiplied by the value at index `n` of `table2`.
*/
pub struct Phasor<'a> {
    // system: Arc<System>,
    // Wavetable reference
    table: &'a Wavetable,
    // Fixed-point phase, with 16 fractional bits
    phase: Wrapping<i32>,
    // Converts radial phase values to table index increments
    radtoinc: f32,
    // Converts frequency (in cycles per second) to table index increments per output samples
    cpstoinc: f32,
    // sampledur: f32
}

impl Wavetable {
    /** Creates a new Wavetable

    # Arguments

    * `table`:     A slice that holds the values for the table. The length must be a power of two and no more than 2^17.

    # Examples

    ```
    # use wavetable::wt::Wavetable;
    // Create 44.1kHz wavetable that ramps from 0 to 128.
    let table = Vec::from_iter((0..128).map(|v| -> f32 {v as f32}));
    let mut wt = Wavetable::new(&table);
    ```
    */
    pub fn new(table: &[f32]) -> Self {
        let size = table.len();
        assert_eq!(
            size & (size - 1),
            0,
            "Wavetable size must be a power of two. Got {}",
            size
        );
        assert!(
            size <= 131072,
            "Phase computation is not precise for wavetables longer than (2**17)"
        );

        let mut wt = Wavetable {
            table1: Vec::with_capacity(size),
            table2: Vec::with_capacity(size),
            lomask: (size - 1) as i32,
        };

        // Create the tables
        for i in 0..(size - 1) {
            let val1 = table[i];
            let val2 = table[i + 1];
            wt.table1.push(2.0 * val1 - val2);
            wt.table2.push(val2 - val1);
        }
        let val1 = table[size - 1];
        let val2 = table[0];
        wt.table1.push(2.0 * val1 - val2);
        wt.table2.push(val2 - val1);
        wt
    }

    pub fn len(&self) -> usize {
        self.table1.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /** Crates a new phasor for this wavetable

    # Arguments

    * `sampledur`: The sampling period (the inverse of the sample rate).
    */
    pub fn new_phasor(&self, system: &Arc<System>) -> Phasor {
        Phasor::new(system, self)
    }

    #[inline]
    fn interpolate(&self, phase: i32) -> f32 {
        let frac = phase_frac1(phase);
        let index = ((phase >> XLOBITS1) & self.lomask) as usize;
        self.table1[index] + (frac * self.table2[index])
    }
}

const XLOBITS1: i32 = 16;

impl<'a> Phasor<'a> {
    fn new(system: &Arc<System>, table: &'a Wavetable) -> Self {
        let sampledur = 1.0 / system.samplerate();

        let size = table.len();
        let sizef32 = size as f32;
        Phasor {
            // system: system.clone(),
            table,
            phase: Wrapping(0),

            // sampledur,
            radtoinc: 65536.0 * sizef32 / (2.0 * PI),
            cpstoinc: sizef32 * sampledur * 65536.0,
        }
    }

    /** Performs the wavetable oscillation operation with audio-rate frequency and/or phase modulation

    # Arguments

    * `outbuf`:  A buffer for storing the output waveform
    * `freqin`:  A sample-by-sample frequency. This must be the same length as outbuf.
    * `phasein`: A sample-by-sample offset phase, useful for phase modulation. This must be the same length as outbuf.

    # Panics

    This function will panic if either the `freqin` or `phasein` buffer lengths are shorter than the `outbuf` length.
    */
    pub fn perform_fm(&mut self, outbuf: &mut [f32], freqin: &[f32], phasein: &[f32]) {
        for i in 0..outbuf.len() {
            let phaseoffset = self.phase + Wrapping((self.radtoinc * phasein[i]) as i32);
            outbuf[i] = self.table.interpolate(phaseoffset.0);
            self.phase += Wrapping((self.cpstoinc * freqin[i]) as i32);
        }
    }

    /** Performs the wavetable oscillation operation with control-rate frequency and/or phase modulation

    # Arguments

    * `outbuf`:  A buffer for storing the output waveform
    * `freqin`:  The frequency (in Hz)
    * `phasein`: The phase offset (in radians)

    # Panics

    This function will panic if either the `freqin` or `phasein` buffer lengths are shorter than the `outbuf` length.
    */
    pub fn perform(&mut self, outbuf: &mut [f32], freqin: f32, phasein: f32) {
        for out in outbuf {
            let phaseoffset = self.phase + Wrapping((self.radtoinc * phasein) as i32);
            *out = self.table.interpolate(phaseoffset.0);
            self.phase += Wrapping((self.cpstoinc * freqin) as i32);
        }
    }
}

#[repr(C)]
union PhaseConv {
    iphase: i32,
    fphase: f32,
}

#[inline]
fn phase_frac1(phase: i32) -> f32 {
    let p = PhaseConv {
        iphase: 0x3F800000 | (0x007FFF80 & ((phase) << 7)),
    };
    unsafe { p.fphase }
}

#[cfg(test)]
mod tests {
    use super::super::system::System;
    use super::Wavetable;
    use float_cmp::approx_eq;
    use std::f32::consts::PI;
    use std::sync::Arc;

    fn generate_ramp(len: usize) -> Vec<f32> {
        Vec::from_iter((0..len).map(|v| -> f32 { v as f32 }))
    }

    #[test]
    fn test_create_wavetable() {
        let table = generate_ramp(128);
        let _wt = Wavetable::new(&table);
    }

    #[test]
    #[should_panic(expected = "Wavetable size must be a power of two. Got 127")]
    fn test_create_wavetable_bad() {
        let table = generate_ramp(127);
        let _wt = Wavetable::new(&table);
    }

    #[test]
    fn test_phasor() {
        //! This will produce an output that rises steadily until it reaches 127 ,at the 1017th sample,
        //! and will then interpolate downward to zero at the 1025th sample.
        let fs = 1024.0;
        let system = Arc::new(System::new(1024.0, 1));
        let table_len = 128;

        let table = generate_ramp(table_len);
        let wt = Wavetable::new(&table);
        let mut phasor = wt.new_phasor(&system);

        let freq = 1.0;
        let phase = 0.0;
        let mut outbuf = [0.0; 1025];
        phasor.perform(&mut outbuf, freq, phase);

        let samples_per_index = (fs as usize) / table_len;

        // The output should rise until the index hits table_len - 1
        let rise_samples = (fs as usize) - samples_per_index;

        for (i, v) in outbuf.iter().enumerate() {
            let expected = if i <= rise_samples {
                (table_len * i) as f32 / fs
            } else {
                (table_len - 1) as f32
                    * (1.0 - ((i - rise_samples) as f32) / (samples_per_index as f32))
            };
            assert!(
                approx_eq!(f32, *v, expected, epsilon = 1e-3),
                "out[{}] = {}, expected: {}, diff: {}",
                i,
                *v,
                expected,
                num::abs(*v - expected)
            );
        }
    }

    #[test]
    fn test_multi_phasors() {
        //! This will produce an output that rises steadily until it reaches 127 ,at the 1017th sample,
        //! and will then interpolate downward to zero at the 1025th sample.

        let fs = 1024.0;
        let system = Arc::new(System::new(1024.0, 1));
        let table_len = 128;
        let peak = (table_len - 1) as f32;

        let table = generate_ramp(table_len);
        let wt = Wavetable::new(&table);
        let mut phasor1 = wt.new_phasor(&system);
        let mut phasor2 = wt.new_phasor(&system); // The second phasor will run with +pi phase
        let mut phasor3 = wt.new_phasor(&system); // The 3d phasor will run at twice the frequency

        let mut outbuf1 = [0.0f32; 1025];
        let mut outbuf2 = [0.0f32; 1025];
        let mut outbuf3 = [0.0f32; 513];

        phasor1.perform(&mut outbuf1, 1.0, 0.0);
        phasor2.perform(&mut outbuf2, 1.0, PI);
        phasor3.perform(&mut outbuf3, 2.0, 0.0);

        // ----------------- Test phasor 1 -----------------------

        let samples_per_index = (fs as usize) / table_len;

        // The output should rise until the index hits table_len - 1
        let rise_samples = (fs as usize) - samples_per_index;

        for (i, v) in outbuf1.iter().enumerate() {
            let expected = if i <= rise_samples {
                let slope = table_len as f32 / fs;
                slope * i as f32
            } else {
                let slope = -peak / samples_per_index as f32;
                let start = rise_samples;
                peak + slope * ((i - start) as f32)
            };
            assert!(
                approx_eq!(f32, *v, expected, epsilon = 1e-3),
                "out[{}] = {}, expected: {}, diff: {}",
                i,
                *v,
                expected,
                num::abs(*v - expected)
            );
        }

        // ----------------- Test phasor 2 -----------------------

        let rise_samples = (fs as usize) / 2 - samples_per_index;

        for (i, v) in outbuf2.iter().enumerate() {
            let expected = if i <= rise_samples {
                let slope = table_len as f32 / fs;
                let start = (table_len / 2) as f32;
                start + slope * i as f32
            } else if i <= rise_samples + samples_per_index {
                let slope = -peak / samples_per_index as f32;
                let start = rise_samples;
                peak + slope * ((i - start) as f32)
            } else {
                let slope = table_len as f32 / fs;
                let start = rise_samples + samples_per_index;
                slope * (i - start) as f32
            };
            assert!(
                approx_eq!(f32, *v, expected, epsilon = 1e-3),
                "out[{}] = {}, expected: {}, diff: {}",
                i,
                *v,
                expected,
                num::abs(*v - expected)
            );
        }

        // ----------------- Test phasor 3 -----------------------

        let fs = fs / 2.0; // Halving the sample rate is the same as doubling the phasor frequency
        let samples_per_index = (fs as usize) / table_len;

        // The output should rise until the index hits table_len - 1
        let rise_samples = (fs as usize) - samples_per_index;

        for (i, v) in outbuf3.iter().enumerate() {
            let expected = if i <= rise_samples {
                let slope = table_len as f32 / fs;
                slope * i as f32
            } else {
                let slope = -peak / samples_per_index as f32;
                let start = rise_samples;
                peak + slope * ((i - start) as f32)
            };
            assert!(
                approx_eq!(f32, *v, expected, epsilon = 1e-3),
                "out[{}] = {}, expected: {}, diff: {}",
                i,
                *v,
                expected,
                num::abs(*v - expected)
            );
        }
    }
}
