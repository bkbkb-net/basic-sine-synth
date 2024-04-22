use nih_plug::{params::smoothing::Smoother, util};

#[derive(Clone, Debug)]
pub struct Sine {
    // id: u64,
    pub sampling_rate: f32,
    pub curr_sample: u32,

    // Midi state
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    // internal_voice_id: u64,
    pub velocity_sqrt: f32,

    pub releasing: bool,

    pub voice_gain: Option<(f32, Smoother<f32>)>,
}

impl Sine {
    pub const fn new(
        // id: u64,
        sampling_rate: f32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) -> Self {
        Self {
            // id,
            sampling_rate,
            curr_sample: 0,

            voice_id,
            channel,
            note,
            velocity_sqrt: 1.0,

            releasing: false,

            voice_gain: None,
        }
    }

    pub fn calculate_sine(&mut self) -> f32 {
        let sine = (self.curr_sample as f32 / self.sampling_rate
            * util::f32_midi_note_to_freq(self.note as f32)
            * std::f32::consts::TAU)
            .sin();

        self.curr_sample += 1;

        sine
    }
}
