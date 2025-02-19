use rebicycler::ReBiCycler;
use rust_sc2::prelude::*;

#[test]
fn assembles_initial_state_properly() {
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
            Difficulty::VeryEasy,
            None, // AI Build (random here)
        ),
        "AutomatonLE", // Map name
        options,
    ).unwrap();
    
}