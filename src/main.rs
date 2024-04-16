use basic_sine_synth::BasicSineSynth;
use nih_plug::prelude::*;

fn main() {
    nih_export_standalone::<BasicSineSynth>();
}
