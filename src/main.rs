use rust_sc2::prelude::*;
use rebicycler::ReBiCycler;


fn main() -> SC2Result<()> {
    let mut bot = ReBiCycler::new();
    let options = LaunchOptions::<'_>{
        realtime: false,
        ..Default::default()
    };

    println!("{}", options.realtime);
    run_vs_computer(
        &mut bot,
        Computer::new(
            Race::Random,
            Difficulty::VeryHard,
            None, // AI Build (random here)
        ),
        "AutomatonLE", // Map name
        options,
    )
}
