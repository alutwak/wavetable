use rustfft::{FftPlanner, num_complex::Complex};
use std::cmp::Ordering::Equal;

use std::ffi::{CString, CStr};
use sndfile_sys as sndfile;
use sndfile_sys::{SF_INFO, SFM_READ, SNDFILE, sf_count_t};


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

fn harmonics(buffer: &[f32]) -> Vec<(f32, f32)> {

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

    // Open a file in write-only mode, returns `io::Result<File>`
    // let mut file = match std::fs::File::create("freqs") {
    //     Err(why) => panic!("couldn't create freqs: {}", why),
    //     Ok(file) => file,
    // };


    for (i, coef)  in fftbuf[..buffer.len()/2].iter().enumerate() {
        let mag = coef.norm();

        //write!(file, "{}, {}", fs * (i as f32 / buffer.len() as f32), mag).unwrap();
        if  mag > last_mag {
            // mag is greater than local_max. make it the new local max
            was_peak = true;
            //write!(file, " new max").unwrap();
        } else if was_peak {
            // No new local maxes in window. Push next peak and reset
            was_peak = false;
            peaks.push(((i-1) as f32 / len_flt, last_mag));
            //write!(file, " peak").unwrap();
        }
        last_mag = mag;
        //writeln!(file).unwrap();
    }

    peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Equal));

    peaks
}

pub fn fundamental(buffer: &[f32], fs: f32) -> f32 {
    let min_harm = 20.0 / fs;
    let harms = harmonics(buffer);
    for (harm, _ampl) in harms {
        if harm >= min_harm {
            return harm;
        }
    }
    0.0
}
#[cfg(test)]
mod tests {
    use super::{harmonics, read_sndfile, best_waveform};

    fn generate_triangle(len: usize, cps: f32) -> Vec<f32> {
        let slope = 4.0 * cps;
        let cycle = (1.0 / cps) as usize;
        let half_cycle = cycle / 2;
        println!("slope: {}, cycle: {}, half: {}", slope, cycle, half_cycle);
        Vec::from_iter((0..len).map(|i| -> f32 {
            let i = i % cycle;
            if i <= half_cycle {
                slope * (i as f32) - 1.0
            } else {
                1.0 - (slope * ((i - half_cycle) as f32))
            }
        }))
    }

    #[test]
    fn test_harmonics() {
        let fs = 48000.0;
        let signal = generate_triangle(fs as usize * 10, 197.0 / fs);

        let peaks = harmonics(&signal);

        for (i, peak) in peaks[..20].iter().enumerate() {
            println!("peak {}: {} ({})", i + 1, peak.0 * fs, peak.1);
        }
    }

    #[test]
    fn test_harmonics_afile() {
        let (signal, fs) = read_sndfile("test/LongVoice.wav").unwrap();

        let peaks = harmonics(&signal);

        for (i, peak) in peaks[..20].iter().enumerate() {
            println!("peak {}: {} ({})", i + 1, peak.0 * fs as f32, peak.1);
        }
    }
}
