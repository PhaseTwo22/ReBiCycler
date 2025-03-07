use rebicycler::protoss_bot::ReBiCycler;
use rust_sc2::prelude::*;

fn main() -> SC2Result<()> {
    let mut bot = ReBiCycler::new();
    let options = rebicycler::get_options();
    run_vs_computer(
        &mut bot,
        Computer::new(
            Race::Random,
            Difficulty::VeryHard,
            None, // AI Build (random here)
        ),
        "AutomatonAIE", // Map name
        options,
    )
}
