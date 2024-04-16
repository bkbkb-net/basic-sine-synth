pub mod editor;
pub mod sine;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

pub struct BasicSineSynth {
    params: Arc<BasicSineSynthParams>,

    // peak_meter_decay_weight: f32,

    // Sine state
    sine: sine::Sine,
}

#[derive(Params)]
struct BasicSineSynthParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    // Sine params
    #[id = "freq"]
    pub frequency: FloatParam,
}

impl Default for BasicSineSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(BasicSineSynthParams::default()),
            sine: sine::Sine::new(48000.0),
        }
    }
}

impl Default for BasicSineSynthParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            editor_state: editor::default_state(),
            frequency: FloatParam::new(
                "Frequency",
                440.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            // We purposely don't specify a step size here, but the parameter should still be
            // displayed as if it were rounded. This formatter also includes the unit.
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
        }
    }
}

impl Plugin for BasicSineSynth {
    const NAME: &'static str = "Basic Sine Synth";
    const VENDOR: &'static str = "bkbkb networks";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "kabi@bkbkb.net";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            // This is also the default and can be omitted here
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_dbg!(buffer_config.sample_rate);
        self.sine.sampling_rate = buffer_config.sample_rate;

        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            let frequency = self.params.frequency.smoothed.next();
            nih_dbg!(gain, frequency);
            let sine_sample = self.sine.calculate_sine(frequency);
            for sample in channel_samples {
                *sample = sine_sample * gain;
            }
        }

        ProcessStatus::KeepAlive
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }
}

impl ClapPlugin for BasicSineSynth {
    const CLAP_ID: &'static str = "net.bkbkb.basic-sine-synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Just a basic sine synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for BasicSineSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"Basic Sine Synth";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(BasicSineSynth);
nih_export_vst3!(BasicSineSynth);
