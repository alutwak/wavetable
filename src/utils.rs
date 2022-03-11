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
