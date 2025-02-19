use rust_sc2::prelude::*;
use rebicycler::ReBiCycler;


fn main() -> SC2Result<()> {
    let mut bot = ReBiCycler::new();
    let mut options = LaunchOptions::default();
    options.realtime = false;
    println!("{}", options.realtime);
    run_vs_computer(
        &mut bot,
        Computer::new(
            Race::Random,
            Difficulty::VeryEasy,
            None, // AI Build (random here)
        ),
        "AutomatonLE", // Map name
        options,
    )
}
