pub struct Sine {
    pub sampling_rate: f32,
    pub curr_sample: u32,
}

impl Sine {
    pub fn new(sampling_rate: f32) -> Self {
        Self {
            sampling_rate,
            curr_sample: 0,
        }
    }
    pub fn calculate_sine(&mut self, frequency: f32) -> f32 {
        let sine =
            (self.curr_sample as f32 / self.sampling_rate * frequency * std::f32::consts::TAU)
                .sin();

        self.curr_sample += 1;

        sine
    }
}
