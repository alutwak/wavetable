use rustfft::{FftPlanner, num_complex::Complex};
use std::cmp::Ordering::Equal;
use num::FromPrimitive;
use std::ffi::{CString, CStr};
use sndfile_sys as sndfile;
use sndfile_sys::{SF_INFO, SFM_READ, SNDFILE, sf_count_t};

/** Reads an audio file and returns the audio in it as a vector

If the audio file has multiple tracks then these tracks are mixed together into a single track

# Arguments

* `path`: The path to the audio file
*/
pub  fn read_sndfile(path: &str) -> Result<(Vec<f32>, i32), std::io::Error> {
        let mut info = SF_INFO {
            frames: 0,
            samplerate: 0,
            channels: 0,
            format: 0,
            sections: 0,
            seekable: 0
        };
        let c_path = CString::new(path).unwrap();
        let sf: *mut SNDFILE = unsafe { sndfile::sf_open(c_path.as_ptr() as *mut _, SFM_READ, &mut info) };
        if sf as usize == 0 {
            let reason_pchar = unsafe { sndfile::sf_strerror(sf) };
            let reason = unsafe { CStr::from_ptr(reason_pchar).to_str().unwrap() };
            return Err( std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to open {}. {}", path, reason))
            );
        }

        let tablelen = (info.frames as f32).log2().floor().exp2() as usize;
        let mut table = Vec::<f32>::with_capacity(tablelen * info.channels as usize);
        let count = unsafe { sndfile::sf_readf_float(sf, table.as_mut_ptr(), tablelen as sf_count_t) };
        unsafe { sndfile::sf_close(sf) };

        if count as usize != tablelen {
            return Err( std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Read fewer frames than expected. Expected {}, got {}", tablelen, count)
            ));
        }

        // Tell table how many values it's holding
        unsafe {table.set_len(tablelen * info.channels as usize) };

        // Mix all channels down to a single channel by averaging them
        if info.channels > 1 {
            let chans = info.channels as usize;
            for i in 0..tablelen {
                let mut mixed = 0.0f32;
                for c in 0..chans {
                    mixed += table[(i * chans) + c] / (chans as f32);
                }
                table[i] = mixed;
            }
            table.truncate(tablelen);
        }
    Ok((table, info.samplerate))
}

/** Finds all of the frequency peaks in an audio buffer and returns them

# Returns
A vector of pairs, in which the first value is the peak frequency and the second is the amplitude
*/
fn frequency_peaks(buffer: &[f32], threshold: f32) -> Vec<(f32, f32)> {

    let len_flt = buffer.len() as f32;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());

    let mut fftbuf = vec![Complex{re: 0.0, im: 0.0}; buffer.len()];

    for (re, coef) in buffer.iter().zip(fftbuf.iter_mut()) {
        coef.re = *re;
    }

    fft.process(&mut fftbuf);

    let mut peaks = Vec::new();
    let mut last_mag = 0.0;
    let mut was_peak = false;
    let norm =  (buffer.len() as f32).sqrt();

    for (i, coef)  in fftbuf[..buffer.len()/2].iter().enumerate() {
        let mag = coef.norm() / norm;

        if  mag > last_mag {
            // mag is greater than local_max. make it the new local max
            was_peak = true;
        } else if was_peak {
            // No new local maxes in window. Push next peak and reset
            was_peak = false;
            if last_mag > threshold {
                peaks.push(((i-1) as f32 / len_flt, last_mag));
            }
        }
        last_mag = mag;
    }

    peaks
}

/** Returns the fundamental frequency in the given audio buffer

TODO: The fundamental is defined as the highest-amplitude frequency. It may turn out that this is a flawed approach
for some signals, in which case a more complex algorithm should be used.
*/
pub fn fundamental(buffer: &[f32]) -> Option<f32> {
    let min_harm = 1e-4;  // Don't return DC. 1e-4 is about 20Hz for fs of 192kHz
    let sig_energy = signal_energy(buffer);

    /* Threshold of 1/1000th of signal energy
    I hate having to use an arbitrary threshold, but I can't think of any better hueristic
    */
    let mut harms = frequency_peaks(buffer, 1e-3 * sig_energy);
    harms.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Equal));
    for (harm, ampl) in harms {
        println!("Amplitude: {}", ampl);
        if harm >= min_harm {
            return Some(harm);
        }
    }
    None
}

/** Returns the total energy of the given signal
*/
pub fn signal_energy(buffer: &[f32]) -> f32 {
    buffer.iter().fold(0.0, |sum, v| sum + v*v)
}

/** Returns the root-mean-square amplitude of the given audio signal
*/
pub fn rms(buffer: &[f32]) -> f32 {
    let rm = signal_energy(buffer) / buffer.len() as f32;
    rm.sqrt()
}

/** Returns the best (ie, loudest) cycle of the fundamental frequency in the given audio buffer

# Returns
A slice of the buffer with the best single cycle if one is found, otherwise, None
*/
pub fn best_waveform(buffer: &[f32]) -> Option<&[f32]> {
    let fund = fundamental(buffer)?;
    let spc = (1.0 / fund).round() as usize;

    println!("Fundamental: {} cps", fund);
    println!("Cycle length: {}", spc);

    let mut best_rms = 0.0;
    let mut best_cycle = (0, 0);
    for i in 1..buffer.len() - spc {
        // Check for zero-crossing pairs spc dist away
        if buffer[i] * buffer[i - 1] <= 0.0 &&
            buffer[i + spc - 1] * buffer[i + spc] <= 0.0
        {
            let rms = rms(&buffer[i..i + spc]);
            if rms > best_rms {
                best_cycle = (i, i + spc);
                best_rms = rms;
            }
        }
    }
    println!("waveform: [{}:{}]: {}", best_cycle.0, best_cycle.1, best_rms);

    if best_cycle.0 == 0 && best_cycle.1 == 0 {
        None
    } else {
        Some(&buffer[best_cycle.0..best_cycle.1])
    }
}

/** Performs a linear interpolation on a range of [0:1]
*/
pub fn linear_interp(x: f32, y0: f32, y1: f32) -> f32 {
    y0 + x * (y1 - y0)
}

/** Resamples a buffer, stretching the time so that the returned buffer has the given length

The interpolation at the end of the resampled signal can either wrap back around to the beginning of the
input buffer, or it can hold the final value. Wrapping the signal will usually be more useful for wavetables,
but may be inappropriate for envelopes or other non-cycling signals.
*/
pub fn resample(buffer: &[f32], len: usize, wrap: bool) -> Vec<f32> {
    let t_resamp = (buffer.len() as f32) / (len as f32);
    let inlen = buffer.len();
    Vec::from_iter((0..len).map(|i| -> f32 {
        let phase = (i as f32) * t_resamp;
        let iphase0 = phase.floor() as usize;
        let iphase1 = iphase0 + 1;
        let frac = phase - iphase0 as f32;

        let d0: f32;
        let d1: f32;
        if wrap {
            d0 = buffer[iphase0];
            d1 = buffer[iphase1 % inlen];
        } else {
            d0 = buffer[iphase0];
            if iphase1 > inlen - 1 {
                d1 = buffer[iphase0]; // Hold the last value
            } else {
                d1 = buffer[iphase1];
            }
        }

        linear_interp(frac, d0, d1)
    }))
}

/** Returns the next power of two that is greater than or equal to x
*/
pub fn next_pow_of_2<T>(x: T) -> T
where T: std::ops::Add<Output = T> + num::FromPrimitive + num::ToPrimitive
{
    let xf64 = x.to_f64().unwrap();
    let next_p2 = 2f64.powf(
        xf64.log2().ceil()
    );
    FromPrimitive::from_f64(next_p2).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{frequency_peaks, read_sndfile, best_waveform, signal_energy, resample};
    use rand::{thread_rng, Rng};
    use float_cmp::approx_eq;

    fn generate_triangle(len: usize, cps: f32) -> Vec<f32> {
        let slope = 4.0 * cps;

        // use y = m*x line, and reflect it between y=-1 and y=1
        Vec::from_iter((0..len).map(|x| -> f32 {
            let y = slope * x as f32;
            let reflections = ((y + 1.0) / 2.0).floor();
            if (reflections as u32) % 2u32 == 1 {
                (2.0 * reflections) - y
            } else {
                (-2.0 * reflections) + y
            }
        }))
    }

    fn generate_noise(len: usize) -> Vec<f32> {
        let mut rng = thread_rng();
        Vec::from_iter((0..len).map(move |_| {
            rng.gen_range(-1.0..=1.0)
        }))
    }

    #[test]
    fn test_frequency_peaks() {
        let fs = 48000.0;
        let signal = generate_triangle(fs as usize * 10, 197.0 / fs);

        let sig_energy = signal_energy(&signal);
        let threshold = 1e-6 * sig_energy;
        println!("Threshold: {}", threshold);
        let peaks = frequency_peaks(&signal, threshold);

        println!("Sig energy: {}", sig_energy);
        for peak in peaks[..peaks.len().min(100)].iter() {
            println!("Relative peak height {}: {}", peak.0, peak.1 / sig_energy);
        }

        let exp_fundamental = 197.0 / fs;
        let mut exp_next_peak = exp_fundamental;
        for (i, peak) in peaks[..5].iter().enumerate() {
            println!("peak {}: {} ({})", i, peak.0 * fs, peak.1);
            assert!(
                approx_eq!(f32, peak.0, exp_next_peak, epsilon=0.01 * exp_next_peak),
                "Expected harmonic {}: {}. Got {}", i, exp_next_peak, peak.0
            );
            exp_next_peak += 2.0 * exp_fundamental;
        }
    }

    #[test]
    fn test_freq_peaks_afile() {
        let (signal, fs) = read_sndfile("test/LongVoice.wav").unwrap();

        let sig_energy = signal_energy(&signal);
        let peaks = frequency_peaks(&signal, 1e-3 * sig_energy);

        println!("Sig energy: {}", sig_energy);
        for peak in peaks[..peaks.len().min(100)].iter() {
            println!("Relative peak height {}: {}", peak.0, peak.1 / sig_energy);
        }

        let exp_fundamental = 94.0 / fs as f32;
        for (i, peak) in peaks[..peaks.len().min(20)].iter().enumerate() {
            // Check if the ratio of peak freq to expected fundamental is about integral
            let ratio = peak.0 / exp_fundamental;
            println!("peak {}: {} ({}) -> ratio {} ({})",
                     i, peak.0 * fs as f32, peak.1, ratio, ratio - ratio.round()
            );
            assert!(
                approx_eq!(f32, ratio - ratio.round(), 0.0, epsilon=0.05),
                "Harmonic {}: expected nearly integral ratio. Got ratio of {}", i, ratio,
            );
        }
    }

    #[test]
    fn test_freq_peaks_noise() {
        let signal = generate_noise(2_usize.pow(18));
        let sig_energy = signal_energy(&signal);
        let peaks = frequency_peaks(&signal, 1e-3 * sig_energy);
        assert!(
            peaks.is_empty(),
            "Expected to find no peaks in noise signal. Found: {}", peaks.len()
        );
    }

    #[test]
    fn test_best_waveform() {
        let fs = 48000.0;
        let cps = 197.0 / fs;
        let signal = generate_triangle(fs as usize * 10, cps);
        let slope = 4.0 * cps;

        let wf = best_waveform(&signal).unwrap();

        // Expect waveform to start near 0, and go through one cycle and end at 0
        println!("Triangle waveform len: {}", wf.len());
        assert!(
            approx_eq!(f32, wf[0], 0.0, epsilon=slope),
            "Expected waveform to start at ~0.0, started at {}", wf[0]
        );

        let mut last_dir = (wf[1] - wf[0]).signum();
        let mut dir_changes = 0;
        for i in 1..wf.len() {
            //println!("{}", wf[i]);
            let dir = (wf[i] - wf[i-1]).signum();
            if dir != last_dir {
                // Don't check the slope when the direction changes
                last_dir = dir;
                dir_changes += 1;
            }
        }

        assert!(dir_changes == 2, "Expected exactly one cycle, got {}", dir_changes as f32 / 2.0);
        assert!(
            approx_eq!(f32, wf[wf.len()-1], 0.0, epsilon=slope),
            "Expected waveform to end at ~0.0, started at {}", wf[wf.len()-1]
        );
    }

    #[test]
    fn test_best_waveform_noise() {
        let signal = generate_noise(2_usize.pow(18));
        let wf = best_waveform(&signal);
        assert!(wf.is_none(), "Incorrectly captured a waveform from noise");
    }

    #[test]
    fn test_resample_long() {
        let inlen = 44100;
        let outlen = 48000;
        let infreq = 441.0 / inlen as f32;
        let outfreq = (infreq * inlen as f32) / outlen as f32;

        let signal = generate_triangle(inlen, infreq);
        let resamp = resample(&signal, outlen, true);

        assert!(resamp.len() == outlen);

        let control = generate_triangle(outlen, outfreq);
        for (i, (ctl, tst)) in control.iter().zip(resamp.iter()).enumerate() {
            assert!(
                approx_eq!(f32, *ctl, *tst, epsilon=1e-3)  ,
                "Expected sample {} value: {}. Got {}", i, ctl, tst
            );
        }

    }

    #[test]
    fn test_resample_short() {
        let inlen = 48000;
        let outlen = 44100;
        let infreq = 480.0 / inlen as f32;
        let outfreq = (infreq * inlen as f32) / outlen as f32;

        let signal = generate_triangle(inlen, infreq);
        let resamp = resample(&signal, outlen, true);

        assert!(resamp.len() == outlen);

        let control = generate_triangle(outlen, outfreq);
        for (i, (ctl, tst)) in control.iter().zip(resamp.iter()).enumerate() {
            assert!(
                approx_eq!(f32, *ctl, *tst, epsilon=1e-3)  ,
                "Expected sample {} value: {}. Got {}", i, ctl, tst
            );
        }

    }

}
