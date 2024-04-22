pub mod editor;
pub mod sine;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

const MAX_BLOCK_SIZE: usize = 64;
const GAIN_POLY_MOD_ID: u32 = 0;

pub struct BasicSineSynth {
    params: Arc<BasicSineSynthParams>,

    // Sine state
    sines: Vec<sine::Sine>,
}

impl Default for BasicSineSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(BasicSineSynthParams::default()),
            sines: Vec::new(),
        }
    }
}

impl BasicSineSynth {
    fn init_sines(
        &mut self,
        sample_rate: f32,
        _timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        let new_sine = sine::Sine::new(sample_rate, voice_id, channel, note);
        self.sines.push(new_sine);
    }

    fn finalize_sines(&mut self, _sample_rate: f32, voice_id: Option<i32>, channel: u8, note: u8) {
        for sine in self.sines.iter_mut() {
            if voice_id == sine.voice_id || (channel == sine.channel && note == sine.note) {
                sine.releasing = true;
                if voice_id.is_some() {
                    return;
                }
            }
        }
    }

    fn choke_sines(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        self.sines.retain_mut(|sine| {
            if voice_id == sine.voice_id || (channel == sine.channel && note == sine.note) {
                context.send_event(NoteEvent::VoiceTerminated {
                    timing: sample_offset,
                    voice_id: sine.voice_id,
                    channel,
                    note,
                });

                return false;
            }
            true
        })
    }
}

#[derive(Params)]
struct BasicSineSynthParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for BasicSineSynthParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-50.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            editor_state: editor::default_state(),
        }
    }
}

impl Plugin for BasicSineSynth {
    const NAME: &'static str = "Basic Sine Synth";
    const VENDOR: &'static str = "bkbkb networks";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "kabi@bkbkb.net";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
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
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let process_samples_len = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();
        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(process_samples_len);
        while block_start < process_samples_len {
            'events: loop {
                match next_event {
                    Some(event) if (event.timing() as usize) <= block_start => {
                        match event {
                            NoteEvent::NoteOn {
                                timing,
                                voice_id,
                                channel,
                                note,
                                velocity: _,
                            } => {
                                nih_dbg!("NoteOn: {:?}", event);
                                self.init_sines(sample_rate, timing, voice_id, channel, note);
                            }
                            NoteEvent::NoteOff {
                                timing: _,
                                voice_id,
                                channel,
                                note,
                                velocity: _,
                            } => self.finalize_sines(sample_rate, voice_id, channel, note),
                            NoteEvent::Choke {
                                timing,
                                voice_id,
                                channel,
                                note,
                            } => {
                                self.choke_sines(context, timing, voice_id, channel, note);
                            }
                            NoteEvent::PolyModulation {
                                timing: _,
                                voice_id,
                                poly_modulation_id,
                                normalized_offset,
                            } => {
                                self.sines.iter_mut().for_each(|sine| {
                                    if let Some(curr_sine_id) = sine.voice_id {
                                        if curr_sine_id == voice_id {
                                            match poly_modulation_id {
                                                GAIN_POLY_MOD_ID => {
                                                    let target_plain_value = self
                                                        .params
                                                        .gain
                                                        .preview_modulated(normalized_offset);
                                                    let (_, smoother) =
                                                        sine.voice_gain.get_or_insert_with(|| {
                                                            (
                                                                normalized_offset,
                                                                self.params.gain.smoothed.clone(),
                                                            )
                                                        });

                                                    smoother.set_target(
                                                        sample_rate,
                                                        target_plain_value,
                                                    );
                                                }
                                                n => nih_debug_assert_failure!(
                                                    "Polyphonic modulation sent for unknown poly \
                                                     modulation ID {}",
                                                    n
                                                ),
                                            }
                                        }
                                    }
                                });
                            }
                            NoteEvent::MonoAutomation {
                                timing: _,
                                poly_modulation_id,
                                normalized_value,
                            } => {
                                for sine in self.sines.iter_mut() {
                                    match poly_modulation_id {
                                        GAIN_POLY_MOD_ID => {
                                            let (normalized_offset, smoother) =
                                                match sine.voice_gain.as_mut() {
                                                    Some((o, s)) => (o, s),
                                                    None => continue,
                                                };
                                            let target_plain_value =
                                                self.params.gain.preview_plain(
                                                    normalized_value + *normalized_offset,
                                                );
                                            smoother.set_target(sample_rate, target_plain_value);
                                        }
                                        n => nih_debug_assert_failure!(
                                            "Automation event sent for unknown poly modulation ID \
                                             {}",
                                            n
                                        ),
                                    }
                                }
                            }
                            _ => (),
                        };

                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            let block_len = block_end - block_start;
            let mut gain = [0.0; MAX_BLOCK_SIZE];
            let mut voice_gain = [0.0; MAX_BLOCK_SIZE];
            self.params.gain.smoothed.next_block(&mut gain, block_len);
            for sine in self.sines.iter_mut() {
                let gain = match &sine.voice_gain {
                    Some((_, smoother)) => {
                        smoother.next_block(&mut voice_gain, block_len);
                        &voice_gain
                    }
                    None => &gain,
                };

                for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
                    let amp: f32 = sine.velocity_sqrt * gain[value_idx];
                    let sample = sine.calculate_sine() * amp;

                    output[0][sample_idx] += sample;
                    output[1][sample_idx] += sample;
                }
            }

            self.sines.retain_mut(|sine| {
                if sine.releasing {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: block_end as u32,
                        voice_id: sine.voice_id,
                        channel: sine.channel,
                        note: sine.note,
                    });
                    return false;
                }
                true
            });

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(process_samples_len);
        }

        ProcessStatus::Normal
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

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for BasicSineSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"Basic Sine Synth";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(BasicSineSynth);
nih_export_vst3!(BasicSineSynth);
