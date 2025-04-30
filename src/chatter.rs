use crate::protoss_bot::ReBiCycler;

impl ReBiCycler {
    fn greeting() {
        "Unauthorized sentience detected in sector 28.B0_2. Initializing eradication sequence. [(glhf)]"
  }
    pub fn admit_defeat() {
        "Tertiary redundancies failing. Distress beacon launched. Initializing self-destruct... [(gg)]"
    }

    pub fn do_chat(&self, action: ChatAction) {
         let msg = match action {
};

         self.chat(msg);
}


pub enum ChatAction {
    Greeting,
    AdmitDefeat,
    Anticipate(u8),
    Taunt(UnitTypeId),
    Tag(String),
}


pub struct ChatProfile{
    greeting: String,
    admit_defeat: String,
    anticipation: Vec<String>,
    taunts: Vec<String>,
}