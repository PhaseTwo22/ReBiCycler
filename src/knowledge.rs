impl crate::protoss_bot::ReBiCycler {
    pub fn observe(&mut self) {
        self.state.action_errors.iter().for_each(|error| {
            println!("Action failed: {error:?}");
        });
    }
}
