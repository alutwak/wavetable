/** Stores the system settings
*/
pub struct System {
    // The sample rate to use
    samplerate: f32,

    // The samplerate divided by the control rate
    controlrate_div: f32,

    // The buffer size to use
    bufsize: usize,
}

impl System {
    /** Creates a new System
    
    # Arguments
    
    * `samplerate`: The sample rate at which the system should run

    * `controlrate_div`: Specifies the control rate as the divisor of the sample rate. So if, for instance, the samplerate is
                         48kHz and the desired control rate is 375Hz, this controlrate_div should be 48000/375 = 128. At 
                         this time, this should be equal to bufsize.

    * `bufsize`:         The output streams buffer size. This is the number of frames that will be processed on each 
                         perform callback iteration. Smaller buffer sizes will decrease latency, but if the buffer size is 
                         too small then there is a risk that the stream will underflow.
    */
    pub fn new(samplerate: f32, controlrate_div: u64, bufsize: usize) -> Self {
        System {
            samplerate,
            controlrate_div: controlrate_div as f32,
            bufsize
        }
    }

    /** Returns the System's sample rate
     */
    pub fn samplerate(&self) -> f32 {
        self.samplerate
    }

    /** Returns the System's controlrate divider
     */
    pub fn controlrate_div(&self) -> f32 {
        self.controlrate_div
    }

    /** Returns the System's buffer size
     */
    pub fn bufsize(&self) -> usize {
        self.bufsize
    }
}
